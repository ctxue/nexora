import { Check, Pencil, Plus, Trash2 } from "lucide-react";
import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";

import {
  COMMAND_ICON_CHOICES,
  COMMAND_LIMIT,
  COMMAND_PRESETS,
  buildUrl,
  commandFromPreset,
  commandHost,
  createCommand,
  validateCommand,
  type CommandAction,
  type CommandError,
  type CommandKind,
  type CustomCommand,
} from "../../core/commands";
import { platform, type BrowserInfo } from "../../platform/tauri";
import { useI18n } from "../../i18n";
import { CommandGlyph, CommandGlyphIcon } from "../CommandGlyph";
import { Toggle } from "./Controls";

interface CommandsSettingsProps {
  commands: CustomCommand[];
  onChange: (next: CustomCommand[]) => void;
}

const ERROR_KEYS: Record<Exclude<CommandError, null>, string> = {
  title: "commands.error.title",
  keywordFormat: "commands.error.keywordFormat",
  keywordTaken: "commands.error.keywordTaken",
  template: "commands.error.template",
  templateQuery: "commands.error.templateQuery",
  url: "commands.error.url",
  target: "commands.error.target",
};

const KIND_KEYS: Record<CommandKind, string> = {
  "web-search": "commands.type.webSearch",
  "open-url": "commands.type.openUrl",
  "open-path": "commands.type.openPath",
  launch: "commands.type.launch",
};

function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <label className="command-field">
      <span className="command-field-label">{label}</span>
      {children}
      {hint ? <span className="command-field-hint">{hint}</span> : null}
    </label>
  );
}

export function CommandsSettings({
  commands,
  onChange,
}: CommandsSettingsProps) {
  const { t } = useI18n();
  const [browsers, setBrowsers] = useState<BrowserInfo[]>([]);
  const [draft, setDraft] = useState<CustomCommand | null>(null);
  const [error, setError] = useState<CommandError>(null);
  const editorRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    void platform
      .listBrowsers()
      .then(setBrowsers)
      .catch(() => setBrowsers([]));
  }, []);

  useEffect(() => {
    if (draft) {
      editorRef.current?.scrollIntoView({ block: "nearest" });
    }
  }, [draft]);

  const editing = useMemo(
    () => commands.some((command) => command.id === draft?.id),
    [commands, draft?.id],
  );

  const browserLabel = (browser: string) =>
    browser === "default"
      ? t("commands.browserDefault")
      : (browsers.find((entry) => entry.exe === browser)?.name ?? browser);

  const summary = (command: CustomCommand): string => {
    const action = command.action;
    switch (action.kind) {
      case "web-search":
        return `${command.keyword} + Tab · ${commandHost(command) || action.template}`;
      case "open-url":
        return `${browserLabel(action.browser)} · ${action.target}`;
      case "open-path":
        return action.target;
      case "launch":
        return [action.target, action.args].filter(Boolean).join(" ");
    }
  };

  const startNew = () => {
    setError(null);
    setDraft(createCommand());
  };

  const startEdit = (command: CustomCommand) => {
    setError(null);
    setDraft({
      ...command,
      action: { ...command.action },
    });
  };

  const patch = (values: Partial<CustomCommand>) => {
    setDraft((current) => (current ? { ...current, ...values } : current));
  };

  const patchAction = (values: Partial<CommandAction>) => {
    setDraft((current) =>
      current ? { ...current, action: { ...current.action, ...values } } : current,
    );
  };

  const save = () => {
    if (!draft) return;

    const problem = validateCommand(draft, commands);
    if (problem) {
      setError(problem);
      return;
    }

    const next: CustomCommand = {
      ...draft,
      keyword: draft.keyword.trim().toLowerCase(),
      title: draft.title.trim(),
      subtitle: draft.subtitle.trim(),
      updatedAt: Date.now(),
    };

    onChange(
      editing
        ? commands.map((command) => (command.id === next.id ? next : command))
        : [...commands, next],
    );
    setDraft(null);
    setError(null);
  };

  const remove = (command: CustomCommand) => {
    onChange(commands.filter((entry) => entry.id !== command.id));
    if (draft?.id === command.id) setDraft(null);
  };

  const toggle = (command: CustomCommand) => {
    onChange(
      commands.map((entry) =>
        entry.id === command.id ? { ...entry, enabled: !entry.enabled } : entry,
      ),
    );
  };

  const addPreset = (preset: (typeof COMMAND_PRESETS)[number]) => {
    if (commands.length >= COMMAND_LIMIT) return;
    if (
      commands.some(
        (command) => command.keyword.toLowerCase() === preset.keyword,
      )
    ) {
      return;
    }
    onChange([...commands, commandFromPreset(preset)]);
  };

  const pickInto = async (
    picker: () => Promise<string | null>,
  ) => {
    try {
      const path = await picker();
      if (path) patchAction({ target: path });
    } catch {
      // Cancelled or unavailable; the field keeps its value.
    }
  };

  return (
    <div className="commands-page">
      <div className="commands-intro">
        <div className="settings-description">{t("settings.commandsHint")}</div>
        <button
          type="button"
          className="settings-action"
          onClick={startNew}
          disabled={Boolean(draft) || commands.length >= COMMAND_LIMIT}
        >
          <Plus size={13} strokeWidth={2} />
          {t("commands.add")}
        </button>
      </div>

      {commands.length === 0 && !draft ? (
        <div className="commands-empty">{t("commands.empty")}</div>
      ) : null}

      {commands.length > 0 ? (
        <div className="commands-list">
          {commands.map((command) => (
            <div
              key={command.id}
              className={`command-row${command.enabled ? "" : " is-disabled"}`}
            >
              <CommandGlyph icon={command.icon} />

              <div className="command-copy">
                <div className="command-title-line">
                  <span className="command-title">{command.title}</span>
                  <span className="command-keyword">{command.keyword}</span>
                </div>
                <div className="command-subtitle">{summary(command)}</div>
              </div>

              <div className="command-row-actions">
                <Toggle
                  checked={command.enabled}
                  onClick={() => toggle(command)}
                  label={command.title}
                />
                <button
                  type="button"
                  className="command-icon-button"
                  onClick={() => startEdit(command)}
                  aria-label={t("commands.edit")}
                  title={t("commands.edit")}
                >
                  <Pencil size={12} strokeWidth={2} />
                </button>
                <button
                  type="button"
                  className="command-icon-button is-danger"
                  onClick={() => remove(command)}
                  aria-label={t("commands.delete")}
                  title={t("commands.delete")}
                >
                  <Trash2 size={12} strokeWidth={2} />
                </button>
              </div>
            </div>
          ))}
        </div>
      ) : null}

      <div className="commands-presets">
        <span className="commands-presets-label">{t("commands.presets")}</span>
        {COMMAND_PRESETS.map((preset) => {
          const added = commands.some(
            (command) => command.keyword.toLowerCase() === preset.keyword,
          );
          return (
            <button
              key={preset.id}
              type="button"
              className="command-preset"
              disabled={added || commands.length >= COMMAND_LIMIT}
              onClick={() => addPreset(preset)}
            >
              {added ? (
                <Check size={11} strokeWidth={2.4} />
              ) : (
                <Plus size={11} strokeWidth={2.4} />
              )}
              {preset.title}
            </button>
          );
        })}
      </div>

      {draft ? (
        <div className="command-editor" ref={editorRef}>
          <div className="command-editor-header">
            <span className="command-editor-title">
              {editing ? t("commands.edit") : t("commands.add")}
            </span>
            <div className="command-editor-header-actions">
              <button
                type="button"
                className="command-text-button"
                onClick={() => {
                  setDraft(null);
                  setError(null);
                }}
              >
                {t("commands.cancel")}
              </button>
              <button type="button" className="settings-action" onClick={save}>
                {t("commands.save")}
              </button>
            </div>
          </div>

          <div className="command-editor-grid">
            <Field label={t("commands.field.title")}>
              <input
                className="command-input"
                value={draft.title}
                onChange={(event) => patch({ title: event.target.value })}
                placeholder={t("commands.field.titlePlaceholder")}
                autoFocus
              />
            </Field>

            <Field
              label={t("commands.field.keyword")}
              hint={t("commands.field.keywordHint")}
            >
              <input
                className="command-input"
                value={draft.keyword}
                onChange={(event) => patch({ keyword: event.target.value })}
                placeholder="bing"
                spellCheck={false}
              />
            </Field>

            <Field label={t("commands.field.kind")}>
              <select
                className="command-input"
                value={draft.action.kind}
                onChange={(event) =>
                  patchAction({ kind: event.target.value as CommandKind })
                }
              >
                {(Object.keys(KIND_KEYS) as CommandKind[]).map((kind) => (
                  <option key={kind} value={kind}>
                    {t(KIND_KEYS[kind])}
                  </option>
                ))}
              </select>
            </Field>

            {draft.action.kind === "web-search" ? (
              <Field
                label={t("commands.field.template")}
                hint={t("commands.field.templateHint")}
              >
                <input
                  className="command-input"
                  value={draft.action.template}
                  onChange={(event) =>
                    patchAction({ template: event.target.value })
                  }
                  placeholder="https://cn.bing.com/search?q={query}"
                  spellCheck={false}
                />
              </Field>
            ) : null}

            {draft.action.kind === "open-url" ? (
              <Field label={t("commands.field.url")}>
                <input
                  className="command-input"
                  value={draft.action.target}
                  onChange={(event) =>
                    patchAction({ target: event.target.value })
                  }
                  placeholder="https://example.com"
                  spellCheck={false}
                />
              </Field>
            ) : null}

            {draft.action.kind === "open-path" ? (
              <Field label={t("commands.field.path")}>
                <div className="command-path-row">
                  <input
                    className="command-input"
                    value={draft.action.target}
                    onChange={(event) =>
                      patchAction({ target: event.target.value })
                    }
                    placeholder="C:\Projects"
                    spellCheck={false}
                  />
                  <button
                    type="button"
                    className="settings-action"
                    onClick={() => void pickInto(() => platform.pickFile())}
                  >
                    {t("commands.pickFile")}
                  </button>
                  <button
                    type="button"
                    className="settings-action"
                    onClick={() => void pickInto(() => platform.pickFolder())}
                  >
                    {t("commands.pickFolder")}
                  </button>
                </div>
              </Field>
            ) : null}

            {draft.action.kind === "launch" ? (
              <>
                <Field label={t("commands.field.program")}>
                  <div className="command-path-row">
                    <input
                      className="command-input"
                      value={draft.action.target}
                      onChange={(event) =>
                        patchAction({ target: event.target.value })
                      }
                      placeholder="C:\...\code.exe"
                      spellCheck={false}
                    />
                    <button
                      type="button"
                      className="settings-action"
                      onClick={() =>
                        void pickInto(() => platform.pickExecutable())
                      }
                    >
                      {t("commands.pickProgram")}
                    </button>
                  </div>
                </Field>

                <Field
                  label={t("commands.field.args")}
                  hint={t("commands.field.argsHint")}
                >
                  <input
                    className="command-input"
                    value={draft.action.args}
                    onChange={(event) =>
                      patchAction({ args: event.target.value })
                    }
                    placeholder="--new-window {query}"
                    spellCheck={false}
                  />
                </Field>
              </>
            ) : null}

            {draft.action.kind === "web-search" ||
            draft.action.kind === "open-url" ? (
              <>
                <Field label={t("commands.field.browser")}>
                  <select
                    className="command-input"
                    value={draft.action.browser}
                    onChange={(event) =>
                      patchAction({ browser: event.target.value })
                    }
                  >
                    <option value="default">
                      {t("commands.browserDefault")}
                    </option>
                    {browsers.map((browser) => (
                      <option key={browser.exe} value={browser.exe}>
                        {browser.name}
                      </option>
                    ))}
                  </select>
                </Field>

                <Field label={t("commands.field.newWindow")}>
                  <Toggle
                    checked={draft.action.newWindow}
                    onClick={() =>
                      patchAction({ newWindow: !draft.action.newWindow })
                    }
                    label={t("commands.field.newWindow")}
                  />
                </Field>
              </>
            ) : null}

            <Field label={t("commands.field.icon")}>
              <div className="command-icon-picker">
                {COMMAND_ICON_CHOICES.map((icon) => (
                  <button
                    key={icon}
                    type="button"
                    className={`command-icon-choice${
                      draft.icon === icon ? " is-active" : ""
                    }`}
                    onClick={() => patch({ icon })}
                    aria-label={icon}
                    aria-pressed={draft.icon === icon}
                  >
                    <CommandGlyphIcon icon={icon} size={13} strokeWidth={2} />
                  </button>
                ))}
              </div>
            </Field>
          </div>

          {draft.action.kind === "web-search" &&
          draft.action.template.includes("{query}") ? (
            <div className="command-preview">
              <span className="command-preview-label">
                {t("commands.preview")}
              </span>
              <span className="command-preview-value">
                {buildUrl(draft.action.template, "nexora")}
              </span>
            </div>
          ) : null}

          {error ? (
            <div className="command-error" role="alert">
              {t(ERROR_KEYS[error])}
            </div>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
