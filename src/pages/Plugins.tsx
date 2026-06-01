import { useState, useEffect } from "react";
import { pluginList, pluginToggle, type PluginEntry } from "../lib/api";

function Plugins() {
  const [plugins, setPlugins] = useState<PluginEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [changed, setChanged] = useState(false);

  const load = async () => {
    try {
      const list = await pluginList();
      setPlugins(list);
    } catch (e) {
      setError(String(e));
    }
  };

  useEffect(() => { load(); }, []);

  const handleToggle = async (key: string, enabled: boolean) => {
    setError(null);
    try {
      await pluginToggle(key, enabled);
      setChanged(true);
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

      {error && <div style={{ color: "var(--danger)", marginBottom: 12 }}>{error}</div>}

      {changed && (
        <div
          style={{
            color: "var(--success)",
            marginBottom: 12,
            padding: "8px 12px",
            background: "rgba(34, 197, 94, 0.1)",
            borderRadius: 6,
            fontSize: 12,
          }}
        >
          Config updated. Restart Codex for changes to take effect.
        </div>
      )}

      <div className="section-header">
        <span className="section-title">
          {enabledCount} / {plugins.length} enabled
        </span>
      </div>

      {plugins.length === 0 ? (
        <div className="empty-state">
          <p>No Codex plugins found</p>
          <p style={{ fontSize: 12 }}>Make sure Codex Desktop is installed and config.toml exists</p>
        </div>
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
