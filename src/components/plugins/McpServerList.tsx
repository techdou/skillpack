import { useState } from "react";
import { EmptyState } from "../Feedback";
import type { McpServerEntry } from "../../lib/api";

/**
 * Top-level `[mcp_servers.*]` entries with a SkillPack-private toggle marker,
 * plus remove and add actions. "Enabled" is tracked by SkillPack via an
 * `enabled` field Codex itself ignores, so this is best-effort — Codex still
 * loads the server; disabling only changes what we show.
 */
export function McpServerList({
  servers,
  onToggle,
  onRemove,
  onAdd,
  busy,
}: {
  servers: McpServerEntry[];
  onToggle: (name: string, enabled: boolean) => void;
  onRemove: (name: string) => void;
  onAdd: (entry: {
    name: string;
    type?: string;
    command?: string;
    args: string[];
    url?: string;
  }) => void;
  busy: boolean;
}) {
  const [adding, setAdding] = useState(false);
  const [form, setForm] = useState({
    name: "",
    command: "",
    url: "",
    type: "",
    args: "",
  });

  const resetForm = () =>
    setForm({ name: "", command: "", url: "", type: "", args: "" });

  const submit = () => {
    const name = form.name.trim();
    if (!name) return;
    onAdd({
      name,
      type: form.type.trim() || undefined,
      command: form.command.trim() || undefined,
      url: form.url.trim() || undefined,
      args: form.args
        .split(/\s+/)
        .map((s) => s.trim())
        .filter(Boolean),
    });
    resetForm();
    setAdding(false);
  };

  return (
    <div>
      <div className="section-header">
        <span className="section-title">MCP Servers</span>
        <button className="btn btn-sm btn-primary" disabled={busy} onClick={() => setAdding((v) => !v)}>
          {adding ? "Cancel" : "Add server"}
        </button>
      </div>

      <div className="card-meta" style={{ marginBottom: 12 }}>
        Top-level [mcp_servers] in Codex config. Codex loads all of these;
        disabling here only affects this UI.
      </div>

      {adding ? (
        <div className="card" style={{ marginBottom: 12 }}>
          <div className="kv-row">
            <input
              className="input"
              placeholder="name"
              value={form.name}
              onChange={(e) => setForm({ ...form, name: e.target.value })}
            />
          </div>
          <div className="kv-row">
            <input
              className="input"
              placeholder="command (e.g. npx)"
              value={form.command}
              onChange={(e) => setForm({ ...form, command: e.target.value })}
            />
          </div>
          <div className="kv-row">
            <input
              className="input"
              placeholder="args (space-separated)"
              value={form.args}
              onChange={(e) => setForm({ ...form, args: e.target.value })}
            />
          </div>
          <div className="kv-row">
            <input
              className="input"
              placeholder="url (for http/sse servers)"
              value={form.url}
              onChange={(e) => setForm({ ...form, url: e.target.value })}
            />
          </div>
          <div className="kv-row">
            <input
              className="input"
              placeholder="type (e.g. local, sse)"
              value={form.type}
              onChange={(e) => setForm({ ...form, type: e.target.value })}
            />
          </div>
          <button
            className="btn btn-primary btn-sm"
            disabled={busy || !form.name.trim()}
            onClick={submit}
            style={{ marginTop: 8 }}
          >
            Save
          </button>
        </div>
      ) : null}

      {servers.length === 0 ? (
        <EmptyState
          title="No MCP servers configured"
          hint="Add a server with the button above"
        />
      ) : (
        <div className="mcp-list">
          {servers.map((s) => (
            <div className="mcp-row" key={s.name}>
              <div className="mcp-row-info">
                <span className="mcp-row-name">{s.name}</span>
                <span className="mcp-row-sub">
                  {s.type ? `${s.type} · ` : ""}
                  {s.command || s.url || "—"}
                </span>
                {s.args.length > 0 ? (
                  <span className="mcp-row-path mono">{s.args.join(" ")}</span>
                ) : null}
                {s.env_keys.length > 0 ? (
                  <span className="mcp-row-path">
                    env: {s.env_keys.join(", ")}
                  </span>
                ) : null}
              </div>
              <div className="mcp-row-actions">
                <label className="toggle">
                  <input
                    type="checkbox"
                    role="switch"
                    aria-checked={s.enabled}
                    aria-label={`${s.enabled ? "Disable" : "Enable"} MCP server ${s.name}`}
                    checked={s.enabled}
                    disabled={busy}
                    onChange={(e) => onToggle(s.name, e.target.checked)}
                  />
                  <div className="toggle-track" />
                  <div className="toggle-thumb" />
                </label>
                <button
                  className="btn btn-sm btn-danger"
                  disabled={busy}
                  onClick={() => onRemove(s.name)}
                >
                  Remove
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
