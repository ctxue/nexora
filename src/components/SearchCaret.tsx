import { useCallback, useEffect, useRef, useState } from "react";
import type { RefObject } from "react";

const CARET_WIDTH = 2;
const CARET_HEIGHT = 15;

type CaretPosition = {
  left: number;
  top: number;
  visible: boolean;
};

export function SearchCaret({
  inputRef,
}: {
  inputRef: RefObject<HTMLInputElement | null>;
}) {
  const [position, setPosition] = useState<CaretPosition>({
    left: 0,
    top: 0,
    visible: false,
  });
  const frame = useRef<number | null>(null);
  const canvas = useRef<HTMLCanvasElement | null>(null);

  const measure = useCallback(() => {
    const input = inputRef.current;
    if (
      !input ||
      document.activeElement !== input ||
      input.selectionStart === null ||
      input.selectionStart !== input.selectionEnd
    ) {
      setPosition((current) =>
        current.visible ? { ...current, visible: false } : current,
      );
      return;
    }

    if (!canvas.current) {
      canvas.current = document.createElement("canvas");
    }
    const context = canvas.current.getContext("2d");
    if (!context) return;

    const computed = window.getComputedStyle(input);
    context.font =
      computed.font ||
      `${computed.fontWeight} ${computed.fontSize} ${computed.fontFamily}`;

    const textBeforeCaret = input.value.slice(0, input.selectionStart);
    const textWidth = context.measureText(textBeforeCaret).width;
    const inputLeft = input.offsetLeft;
    const maxLeft = inputLeft + input.clientWidth - CARET_WIDTH;
    const left = Math.max(
      inputLeft,
      Math.min(inputLeft + textWidth - input.scrollLeft, maxLeft),
    );
    const top =
      input.offsetTop + Math.max(0, (input.offsetHeight - CARET_HEIGHT) / 2);

    setPosition({ left, top, visible: true });
  }, [inputRef]);

  const scheduleMeasure = useCallback(() => {
    if (frame.current !== null) {
      window.cancelAnimationFrame(frame.current);
    }
    frame.current = window.requestAnimationFrame(() => {
      frame.current = null;
      measure();
    });
  }, [measure]);

  useEffect(() => {
    const input = inputRef.current;
    if (!input) return;

    const onBlur = () => {
      setPosition((current) => ({ ...current, visible: false }));
    };

    input.addEventListener("focus", scheduleMeasure);
    input.addEventListener("click", scheduleMeasure);
    input.addEventListener("input", scheduleMeasure);
    input.addEventListener("keyup", scheduleMeasure);
    input.addEventListener("keydown", scheduleMeasure);
    input.addEventListener("scroll", scheduleMeasure);
    input.addEventListener("select", scheduleMeasure);
    input.addEventListener("blur", onBlur);
    document.addEventListener("selectionchange", scheduleMeasure);
    window.addEventListener("resize", scheduleMeasure);

    scheduleMeasure();

    return () => {
      input.removeEventListener("focus", scheduleMeasure);
      input.removeEventListener("click", scheduleMeasure);
      input.removeEventListener("input", scheduleMeasure);
      input.removeEventListener("keyup", scheduleMeasure);
      input.removeEventListener("keydown", scheduleMeasure);
      input.removeEventListener("scroll", scheduleMeasure);
      input.removeEventListener("select", scheduleMeasure);
      input.removeEventListener("blur", onBlur);
      document.removeEventListener("selectionchange", scheduleMeasure);
      window.removeEventListener("resize", scheduleMeasure);
      if (frame.current !== null) {
        window.cancelAnimationFrame(frame.current);
      }
    };
  }, [inputRef, scheduleMeasure]);

  return (
    <span
      key={`${Math.round(position.left)}-${Math.round(position.top)}`}
      className={`search-caret${position.visible ? " is-visible" : ""}`}
      style={{
        width: CARET_WIDTH,
        height: CARET_HEIGHT,
        transform: `translate3d(${position.left}px, ${position.top}px, 0)`,
      }}
      aria-hidden="true"
    />
  );
}
