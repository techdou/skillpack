import { useState, useEffect } from "react";
import { ErrorBanner, EmptyState, SuccessBanner } from "../components/Feedback";
import { pluginList, pluginToggle, type PluginEntry } from "../lib/api";

function Plugins() {
  const [plugins, setPlugins] = useState<PluginEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const load = async () => {
    try {
      const list = await pluginList();
      setPlugins(list);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  const handleToggle = async (key: string, enabled: boolean) => {
    setError(null);
    try {
      await pluginToggle(key, enabled);
      setSuccess("Config updated. Restart Codex for changes to take effect.");
      await load();
    } catch (e) {
      setError(String(e));
    }
  };

  const enabledCount = plugins.filter((p) => p.enabled).length;

  return (
    <div>
      <div className="page-title">Plugins</div>
      <div className="card-meta" style={{ marginBottom: 16 }}>
        Codex config.toml plugin management
      </div>

      <ErrorBanner error={error} />
      <SuccessBanner message={success} onDismiss={() => setSuccess(null)} />

      <div className="section-header">
        <span className="section-title">
          {enabledCount} / {plugins.length} enabled
        </span>
      </div>

      {loading ? (
        <div className="card-meta">Loading plugins…</div>
      ) : plugins.length === 0 ? (
        <EmptyState
          title="No Codex plugins found"
          hint="Make sure Codex Desktop is installed and config.toml exists"
        />
      ) : (
        plugins.map((plugin) => (
          <div className="plugin-row" key={plugin.key}>
            <div className="plugin-info">
              <span className="plugin-name">{plugin.name}</span>
              <span className="plugin-source">{plugin.source}</span>
            </div>
            <label className="toggle">
              <input
                type="checkbox"
                role="switch"
                aria-checked={plugin.enabled}
                aria-label={`${plugin.enabled ? "Disable" : "Enable"} plugin ${plugin.name}`}
                checked={plugin.enabled}
                onChange={(e) => handleToggle(plugin.key, e.target.checked)}
              />
              <div className="toggle-track" />
              <div className="toggle-thumb" />
            </label>
          </div>
        ))
      )}
    </div>
  );
}

export default Plugins;
