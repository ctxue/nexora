import { useLayoutEffect, useRef, useState, type ReactNode } from "react";

interface AnimatedHeightProps {
  children: ReactNode;
  className?: string;
  animate?: boolean;
}

export function AnimatedHeight({
  children,
  className = "",
  animate = true,
}: AnimatedHeightProps) {
  const outerRef = useRef<HTMLDivElement>(null);
  const innerRef = useRef<HTMLDivElement>(null);
  const [ready, setReady] = useState(false);

  useLayoutEffect(() => {
    const node = innerRef.current;
    if (!node) return;

    let frame = 0;
    let readyFrame = 0;

    const measure = () => {
      frame = 0;
      if (outerRef.current) {
        outerRef.current.style.height = `${node.getBoundingClientRect().height}px`;
      }
    };

    const observer = new ResizeObserver(() => {
      if (frame) return;
      frame = requestAnimationFrame(measure);
    });

    observer.observe(node);
    measure();

    readyFrame = requestAnimationFrame(() => setReady(true));

    return () => {
      observer.disconnect();
      if (frame) cancelAnimationFrame(frame);
      if (readyFrame) cancelAnimationFrame(readyFrame);
    };
  }, []);

  return (
    <div
      ref={outerRef}
      className={`animated-height${ready && animate ? " is-ready" : ""}${
        className ? ` ${className}` : ""
      }`}
    >
      <div className="animated-height-inner" ref={innerRef}>
        {children}
      </div>
    </div>
  );
}
