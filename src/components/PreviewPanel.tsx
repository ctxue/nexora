import { FileText, Image as ImageIcon, LoaderCircle, X } from "lucide-react";
import { useEffect, useState } from "react";

import { platform, type PreviewData } from "../platform/tauri";
import { useI18n } from "../i18n";

interface PreviewPanelProps {
  path: string;
  onClose: () => void;
}

export function PreviewPanel({ path, onClose }: PreviewPanelProps) {
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
    <div className="preview-layer" onPointerDown={onClose}>
      <section
        className="preview-panel"
        role="dialog"
        aria-modal="true"
        aria-label={t("preview.title")}
        onPointerDown={(event) => event.stopPropagation()}
      >
        <header className="preview-header">
          <span className="preview-kind" aria-hidden="true">
            {preview?.kind === "image" ? (
              <ImageIcon size={15} strokeWidth={1.8} />
            ) : (
              <FileText size={15} strokeWidth={1.8} />
            )}
          </span>
          <span className="preview-heading">
            <span className="preview-title">
              {preview?.title ?? t("preview.loading")}
            </span>
            <span className="preview-metadata">
              {preview?.metadata ?? t("preview.reading")}
            </span>
          </span>
          <button
            type="button"
            className="preview-close"
            onClick={onClose}
            aria-label={t("preview.close")}
            autoFocus
          >
            <X size={14} strokeWidth={2} />
          </button>
        </header>

        <div className="preview-body">
          {error ? (
            <div className="preview-empty">{t("preview.error")}</div>
          ) : !preview ? (
            <div className="preview-empty">
              <LoaderCircle className="spin" size={18} strokeWidth={1.8} />
              <span>{t("preview.loading")}</span>
            </div>
          ) : preview.dataUrl ? (
            <img
              className="preview-image"
              src={preview.dataUrl}
              alt={preview.title}
            />
          ) : preview.content ? (
            <pre className="preview-text">{preview.content}</pre>
          ) : (
            <div className="preview-empty">{t("preview.noEmbedded")}</div>
          )}
        </div>

        {preview?.truncated ? (
          <footer className="preview-footer">{t("preview.truncated")}</footer>
        ) : null}
      </section>
    </div>
  );
}
