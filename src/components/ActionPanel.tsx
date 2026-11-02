import {
  AppWindow,
  Copy,
  Eye,
  FolderOpen,
  Minimize2,
  Shield,
  X,
  type LucideIcon,
} from "lucide-react";
import { useEffect, useRef } from "react";

import type { LauncherItem } from "../core/types";
import type {
  ResultActionDescriptor,
  ResultActionId,
} from "../features/actions/types";
import { useI18n } from "../i18n";
import { ResultIcon } from "./ResultIcon";

interface ActionPanelProps {
  item: LauncherItem;
  actions: ResultActionDescriptor[];
  selectedIndex: number;
  icons: Record<string, string>;
  onSelect: (index: number) => void;
  onRun: (action: ResultActionDescriptor) => void;
  onClose: () => void;
}

const ACTION_ICONS: Record<ResultActionId, LucideIcon> = {
  "copy-path": Copy,
  "reveal-path": FolderOpen,
  "open-with": AppWindow,
  "run-as-admin": Shield,
  "preview-file": Eye,
  "activate-window": AppWindow,
  "minimize-window": Minimize2,
  "close-window": X,
};

export function ActionPanel({
  item,
  actions,
  selectedIndex,
  icons,
  onSelect,
  onRun,
  onClose,
}: ActionPanelProps) {
  const { t } = useI18n();
  const selectedRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    selectedRef.current?.focus({ preventScroll: true });
  }, [selectedIndex]);

  return (
    <div
      className="action-panel-layer"
      onPointerDown={(event) => {
        event.stopPropagation();
        onClose();
      }}
    >
      <section
        className="action-panel"
        role="menu"
        aria-label={t("actions.title")}
        onPointerDown={(event) => event.stopPropagation()}
      >
        <header className="action-panel-header">
          <span className="action-panel-icon">
            <ResultIcon
              item={item}
              icon={item.path ? icons[item.path] : undefined}
            />
          </span>
          <span className="action-panel-copy">
            <span className="action-panel-title">{item.title}</span>
            <span className="action-panel-subtitle">
              {t("actions.title")}
            </span>
          </span>
        </header>

        <div className="action-panel-items">
          {actions.map((action, index) => {
            const Icon = ACTION_ICONS[action.id];
            return (
              <button
                key={action.id}
                ref={index === selectedIndex ? selectedRef : undefined}
                type="button"
                role="menuitem"
                className={`action-panel-item${
                  index === selectedIndex ? " is-selected" : ""
                }${action.danger ? " is-danger" : ""}`}
                onMouseEnter={() => onSelect(index)}
                onClick={() => onRun(action)}
              >
                <Icon size={15} strokeWidth={1.75} aria-hidden="true" />
                <span>{action.label}</span>
                {action.shortcut ? (
                  <kbd className="action-panel-shortcut">{action.shortcut}</kbd>
                ) : index === selectedIndex ? (
                  <kbd className="action-panel-return">↵</kbd>
                ) : null}
              </button>
            );
          })}
        </div>
      </section>
    </div>
  );
}
