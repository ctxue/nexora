import {
  ArrowUpDown,
  Copy,
  CornerDownLeft,
  FileText,
  Image as ImageIcon,
  LoaderCircle,
  X,
} from "lucide-react";
import { useEffect, useState } from "react";

import type { LauncherItem } from "../core/types";
import { platform, type PreviewData } from "../platform/tauri";
import { useI18n } from "../i18n";
import { ResultIcon } from "./ResultIcon";

interface PreviewPanelProps {
  item: LauncherItem;
  path: string;
  onOpen: () => void;
  onCopyPath: () => void;
  onClose: () => void;
}

export function PreviewPanel({
  item,
  path,
  onOpen,
  onCopyPath,
  onClose,
}: PreviewPanelProps) {
  const { t } = useI18n();
  const [preview, setPreview] = useState<PreviewData | null>(null);
  const [error, setError] = useState(false);

  useEffect(() => {
    let active = true;
    setPreview(null);
    setError(false);
    void platform
      .previewFile(path)
      .then((data) => {
        if (active) setPreview(data);
      })
      .catch(() => {
        if (active) setError(true);
      });
    return () => {
      active = false;
    };
  }, [path]);

  return (
    <div
      className="preview-layer"
      onPointerDown={(event) => {
        event.stopPropagation();
        onClose();
      }}
    >
      <section
        className="preview-panel"
        role="dialog"
        aria-modal="true"
        aria-label={t("preview.title")}
        onPointerDown={(event) => event.stopPropagation()}
      >
        <header className="preview-header">
          <span className="preview-file-icon" aria-hidden="true">
            <ResultIcon item={item} />
          </span>
          <span className="preview-heading">
            <span className="preview-title">
              {preview?.title ?? item.title}
            </span>
            <span className="preview-metadata">
              {preview?.metadata ?? t("preview.reading")}
            </span>
          </span>
          <span className="preview-type">
            {item.extension?.toUpperCase().slice(0, 4) || "FILE"}
          </span>
          <button
            type="button"
            className="preview-close"
            onClick={onClose}
            aria-label={t("preview.close")}
            autoFocus
          >
            <X size={15} strokeWidth={2} />
          </button>
        </header>

        <div className="preview-body">
          {error ? (
            <div className="preview-empty">
              <FileText size={22} strokeWidth={1.6} />
              <span>{t("preview.error")}</span>
            </div>
          ) : !preview ? (
            <div className="preview-empty">
              <LoaderCircle className="spin" size={20} strokeWidth={1.8} />
              <span>{t("preview.loading")}</span>
            </div>
          ) : preview.dataUrl ? (
            <img
              className="preview-image"
              src={preview.dataUrl}
              alt={preview.title}
            />
          ) : preview.kind === "text" && preview.content ? (
            <pre className="preview-text">{preview.content}</pre>
          ) : (
            <div className="preview-summary">
              <span className="preview-summary-icon">
                {preview.kind === "image" ? (
                  <ImageIcon size={28} strokeWidth={1.5} />
                ) : (
                  <FileText size={28} strokeWidth={1.5} />
                )}
              </span>
              <span className="preview-summary-title">{preview.title}</span>
              <span className="preview-summary-copy">
                {preview.content ?? t("preview.noEmbedded")}
              </span>
            </div>
          )}
        </div>

        {preview?.truncated ? (
          <div className="preview-truncated">{t("preview.truncated")}</div>
        ) : null}

        <footer className="preview-toolbar">
          <span className="preview-browse-hint">
            <ArrowUpDown size={12} strokeWidth={1.9} />
            {t("preview.browse")}
          </span>
          <span className="preview-toolbar-actions">
            <button type="button" className="preview-tool" onClick={onOpen}>
              <CornerDownLeft size={12} strokeWidth={2} />
              {t("preview.openFile")}
              <kbd>↵</kbd>
            </button>
            <button type="button" className="preview-tool" onClick={onCopyPath}>
              <Copy size={12} strokeWidth={1.9} />
              {t("preview.copyPath")}
              <kbd>Ctrl C</kbd>
            </button>
            <button type="button" className="preview-close-hint" onClick={onClose}>
              {t("preview.closeHint")}
              <kbd>Esc</kbd>
            </button>
          </span>
        </footer>
      </section>
    </div>
  );
}
