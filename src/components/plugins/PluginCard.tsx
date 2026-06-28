import { Pill } from "../Feedback";
import type { PluginEntry } from "../../lib/api";

/**
 * Compact card summarising one plugin. Shows identity (name, version, source,
 * category), a description, a toggle, and an uninstall button. Clicking the
 * body opens the detail panel (handled by the parent via `onOpen`).
 */
export function PluginCard({
  plugin,
  onToggle,
  onRemove,
  onOpen,
  busy,
}: {
  plugin: PluginEntry;
  onToggle: (key: string, enabled: boolean) => void;
  onRemove: (key: string) => void;
  onOpen: (key: string) => void;
  busy: boolean;
}) {
  return (
    <div className="plugin-card">
      <div
        className="plugin-card-body"
        onClick={() => onOpen(plugin.key)}
        role="button"
        tabIndex={0}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            onOpen(plugin.key);
          }
        }}
      >
        <div className="plugin-card-head">
          <span className="plugin-name">{plugin.name}</span>
          {plugin.version ? (
            <span className="plugin-version">v{plugin.version}</span>
          ) : null}
          {!plugin.installed ? (
            <Pill tone="warn" title="Cache directory not found">
              not installed
            </Pill>
          ) : null}
        </div>
        {plugin.description ? (
          <div className="plugin-card-desc">{plugin.description}</div>
        ) : (
          <div className="plugin-card-desc plugin-card-desc-muted">No description</div>
        )}
        <div className="plugin-card-meta">
          {plugin.source ? (
            <span className="plugin-source">@{plugin.source}</span>
          ) : null}
          {plugin.category ? (
            <span className="plugin-cat">{plugin.category}</span>
          ) : null}
          {plugin.bundled_skills.length > 0 ? (
            <span className="plugin-skills-count">
              {plugin.bundled_skills.length} skill{plugin.bundled_skills.length > 1 ? "s" : ""}
            </span>
          ) : null}
        </div>
      </div>
      <div className="plugin-card-actions">
        <label className="toggle">
          <input
            type="checkbox"
            role="switch"
            aria-checked={plugin.enabled}
            aria-label={`${plugin.enabled ? "Disable" : "Enable"} plugin ${plugin.name}`}
            checked={plugin.enabled}
            disabled={busy}
            onChange={(e) => onToggle(plugin.key, e.target.checked)}
          />
          <div className="toggle-track" />
          <div className="toggle-thumb" />
        </label>
        <button
          className="btn btn-sm btn-danger"
          disabled={busy}
          onClick={() => onRemove(plugin.key)}
          title="Uninstall (via codex CLI)"
        >
          Uninstall
        </button>
      </div>
    </div>
  );
}
