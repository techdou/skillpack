import { useState } from "react";
import { EmptyState } from "../Feedback";
import type { MarketplaceEntry } from "../../lib/api";

/**
 * Configured marketplaces (`[marketplaces.*]`) with add (CLI), upgrade (CLI),
 * and remove (CLI) actions, plus an "add marketplace" input.
 */
export function MarketplaceList({
  marketplaces,
  onAdd,
  onUpgrade,
  onRemove,
  busy,
}: {
  marketplaces: MarketplaceEntry[];
  onAdd: (source: string) => void;
  onUpgrade: (name?: string) => void;
  onRemove: (name: string) => void;
  busy: boolean;
}) {
  const [source, setSource] = useState("");

  const submitAdd = () => {
    const trimmed = source.trim();
    if (!trimmed) return;
    onAdd(trimmed);
    setSource("");
  };

  return (
    <div>
      <div className="section-header">
        <span className="section-title">Marketplaces</span>
        <button
          className="btn btn-sm"
          disabled={busy}
          onClick={() => onUpgrade()}
          title="Upgrade all marketplaces (codex plugin marketplace upgrade)"
        >
          Upgrade all
        </button>
      </div>

      {marketplaces.length === 0 ? (
        <EmptyState
          title="No marketplaces configured"
          hint="Add a marketplace source like owner/repo or a git URL"
        />
      ) : (
        <div className="mcp-list">
          {marketplaces.map((m) => (
            <div className="mcp-row" key={m.name}>
              <div className="mcp-row-info">
                <span className="mcp-row-name">{m.name}</span>
                <span className="mcp-row-sub">
                  {m.source || "—"}
                  {m.ref ? ` @ ${m.ref}` : ""}
                </span>
                {m.path ? (
                  <span className="mcp-row-path mono" title={m.path}>
                    {m.path}
                  </span>
                ) : null}
              </div>
              <div className="mcp-row-actions">
                <button
                  className="btn btn-sm"
                  disabled={busy}
                  onClick={() => onUpgrade(m.name)}
                >
                  Upgrade
                </button>
                <button
                  className="btn btn-sm btn-danger"
                  disabled={busy}
                  onClick={() => onRemove(m.name)}
                >
                  Remove
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      <div className="input-row" style={{ marginTop: 12 }}>
        <input
          className="input"
          placeholder="owner/repo or git URL"
          value={source}
          disabled={busy}
          onChange={(e) => setSource(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") submitAdd();
          }}
        />
        <button className="btn btn-primary" disabled={busy || !source.trim()} onClick={submitAdd}>
          Add marketplace
        </button>
      </div>
    </div>
  );
}
