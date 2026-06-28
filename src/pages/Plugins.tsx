import { useState } from "react";
import { ErrorBanner, EmptyState, SuccessBanner } from "../components/Feedback";
import { useAsync } from "../lib/useAsync";
import {
  pluginOverview,
  pluginToggle,
  pluginAdd,
  pluginRemove,
  featuresPluginsToggle,
  marketplaceList,
  marketplaceAdd,
  marketplaceUpgrade,
  marketplaceRemove,
  mcpList,
  mcpToggle,
  mcpRemove,
  mcpAdd,
} from "../lib/api";
import { PluginCard } from "../components/plugins/PluginCard";
import { PluginDetail } from "../components/plugins/PluginDetail";
import { MarketplaceList } from "../components/plugins/MarketplaceList";
import { McpServerList } from "../components/plugins/McpServerList";

type Tab = "installed" | "marketplaces" | "mcp";

function Plugins() {
  const [tab, setTab] = useState<Tab>("installed");
  const [actionError, setActionError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [openKey, setOpenKey] = useState<string | null>(null);

  // One async read for the master switch + plugin list.
  const overview = useAsync(pluginOverview, []);
  const marketplaces = useAsync(marketplaceList, []);
  const servers = useAsync(mcpList, []);

  const reloadAll = async () => {
    await Promise.all([overview.reload(), marketplaces.reload(), servers.reload()]);
  };

  const runAction = async (label: string, fn: () => Promise<unknown>) => {
    setActionError(null);
    setBusy(true);
    try {
      await fn();
      setSuccess(`${label} done. Restart Codex for changes to take effect.`);
      await reloadAll();
    } catch (e) {
      setActionError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const featuresOn = overview.data?.features_plugins_enabled ?? true;
  const plugins = overview.data?.plugins ?? [];
  const enabledCount = plugins.filter((p) => p.enabled).length;
  const openPlugin = plugins.find((p) => p.key === openKey) ?? null;

  return (
    <div>
      <div className="page-title">Codex Extensions</div>
      <div className="card-meta" style={{ marginBottom: 16 }}>
        Manage Codex plugins, marketplaces, and MCP servers in config.toml
      </div>

      <ErrorBanner error={overview.error ?? actionError} />
      <SuccessBanner message={success} onDismiss={() => setSuccess(null)} />

      {/* Master switch banner */}
      <div className={`features-banner ${featuresOn ? "" : "features-banner-off"}`}>
        <div className="features-banner-info">
          <span className="features-banner-title">Plugins feature</span>
          <span className="features-banner-sub">
            {featuresOn
              ? "Codex plugins are enabled"
              : "Plugins are disabled — toggles below have no effect until this is on"}
          </span>
        </div>
        <label className="toggle">
          <input
            type="checkbox"
            role="switch"
            aria-checked={featuresOn}
            aria-label="Toggle Codex plugins feature"
            checked={featuresOn}
            disabled={busy}
            onChange={(e) =>
              runAction("Plugins feature", () => featuresPluginsToggle(e.target.checked))
            }
          />
          <div className="toggle-track" />
          <div className="toggle-thumb" />
        </label>
      </div>

      {/* Tabs */}
      <div className="tabs">
        <button
          className={`tab ${tab === "installed" ? "tab-active" : ""}`}
          onClick={() => setTab("installed")}
        >
          Installed ({plugins.length})
        </button>
        <button
          className={`tab ${tab === "marketplaces" ? "tab-active" : ""}`}
          onClick={() => setTab("marketplaces")}
        >
          Marketplaces
        </button>
        <button
          className={`tab ${tab === "mcp" ? "tab-active" : ""}`}
          onClick={() => setTab("mcp")}
        >
          MCP Servers
        </button>
      </div>

      {tab === "installed" ? (
        <>
          <div className="section-header">
            <span className="section-title">
              {enabledCount} / {plugins.length} enabled
            </span>
            <button className="btn btn-sm btn-primary" disabled={busy} onClick={reloadAll}>
              Refresh
            </button>
          </div>

          {overview.loading ? (
            <div className="card-meta">Loading plugins…</div>
          ) : plugins.length === 0 ? (
            <EmptyState
              title="No Codex plugins found"
              hint="Install plugins with `codex plugin add`, or check that config.toml exists"
            />
          ) : (
            <>
              <div className="plugin-grid">
                {plugins.map((plugin) => (
                  <PluginCard
                    key={plugin.key}
                    plugin={plugin}
                    busy={busy}
                    onToggle={(key, en) =>
                      runAction("Toggle plugin", () => pluginToggle(key, en))
                    }
                    onRemove={(key) =>
                      runAction("Uninstall plugin", () => pluginRemove(key))
                    }
                    onOpen={setOpenKey}
                  />
                ))}
              </div>
              {openPlugin ? (
                <PluginDetail plugin={openPlugin} onClose={() => setOpenKey(null)} />
              ) : null}
            </>
          )}

          <PluginInstallBox
            disabled={busy}
            onInstall={(name, mkt) =>
              runAction("Install plugin", () => pluginAdd(name, mkt))
            }
          />
        </>
      ) : null}

      {tab === "marketplaces" ? (
        <MarketplaceList
          marketplaces={marketplaces.data ?? []}
          busy={busy}
          onAdd={(src) => runAction("Add marketplace", () => marketplaceAdd(src))}
          onUpgrade={(name) =>
            runAction("Upgrade marketplace", () => marketplaceUpgrade(name))
          }
          onRemove={(name) =>
            runAction("Remove marketplace", () => marketplaceRemove(name))
          }
        />
      ) : null}

      {tab === "mcp" ? (
        <McpServerList
          servers={servers.data ?? []}
          busy={busy}
          onToggle={(name, en) =>
            runAction("Toggle MCP server", () => mcpToggle(name, en))
          }
          onRemove={(name) => runAction("Remove MCP server", () => mcpRemove(name))}
          onAdd={(entry) => runAction("Add MCP server", () => mcpAdd(entry))}
        />
      ) : null}
    </div>
  );
}

/** Compact "install a plugin" input, shown at the bottom of the Installed tab. */
function PluginInstallBox({
  disabled,
  onInstall,
}: {
  disabled: boolean;
  onInstall: (name: string, marketplace?: string) => void;
}) {
  const [name, setName] = useState("");
  const [mkt, setMkt] = useState("");

  const submit = () => {
    const trimmed = name.trim();
    if (!trimmed) return;
    onInstall(trimmed, mkt.trim() || undefined);
    setName("");
    setMkt("");
  };

  return (
    <div className="card" style={{ marginTop: 16 }}>
      <div className="section-header">
        <span className="section-title">Install plugin</span>
      </div>
      <div className="card-meta" style={{ marginBottom: 8 }}>
        Runs <code>codex plugin add</code>. Use <code>name@marketplace</code> or
        fill both fields.
      </div>
      <div className="input-row">
        <input
          className="input"
          placeholder="plugin name"
          value={name}
          disabled={disabled}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") submit();
          }}
        />
        <input
          className="input"
          placeholder="marketplace (optional)"
          value={mkt}
          disabled={disabled}
          onChange={(e) => setMkt(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") submit();
          }}
        />
        <button
          className="btn btn-primary"
          disabled={disabled || !name.trim()}
          onClick={submit}
        >
          Install
        </button>
      </div>
    </div>
  );
}

export default Plugins;
