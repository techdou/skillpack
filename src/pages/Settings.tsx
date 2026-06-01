import { useState, useEffect } from "react";
import { configGet, configSet, type AppConfig } from "../lib/api";

function Settings() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  const load = async () => {
    try {
      const c = await configGet();
      setConfig(c);
    } catch (e) {
      setError(String(e));
    }
  };

  useEffect(() => { load(); }, []);

  const handleSave = async () => {
    if (!config) return;
    setError(null);
    try {
      await configSet(config);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      setError(String(e));
    }
  };

  if (!config) return <div>Loading...</div>;

  return (
    <div>
      <div className="page-title">Settings</div>

      {error && <div style={{ color: "var(--danger)", marginBottom: 12 }}>{error}</div>}
      {saved && (
        <div style={{ color: "var(--success)", marginBottom: 12, fontSize: 12 }}>
          Settings saved
        </div>
      )}

      <div className="card">
        <div className="section-header">
          <span className="section-title">Storage</span>
        </div>
        <div style={{ marginBottom: 12 }}>
          <label style={{ display: "block", fontSize: 12, color: "var(--text-secondary)", marginBottom: 4 }}>
            Skill Packs Directory
          </label>
          <input
            className="input"
            value={config.packs_dir}
            onChange={(e) => setConfig({ ...config, packs_dir: e.target.value })}
          />
        </div>
        <div style={{ marginBottom: 12 }}>
          <label style={{ display: "block", fontSize: 12, color: "var(--text-secondary)", marginBottom: 4 }}>
            Codex config.toml Path
          </label>
          <input
            className="input"
            value={config.codex_config_path || ""}
            onChange={(e) => setConfig({ ...config, codex_config_path: e.target.value || null })}
          />
        </div>
      </div>

      <div className="card">
        <div className="section-header">
          <span className="section-title">Default Toolchains</span>
        </div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          {["codex", "agents", "claude", "cursor"].map((t) => {
            const active = config.default_targets.includes(t);
            return (
              <button
                key={t}
                className={`btn btn-sm ${active ? "btn-primary" : ""}`}
                onClick={() => {
                  const targets = active
                    ? config.default_targets.filter((x) => x !== t)
                    : [...config.default_targets, t];
                  setConfig({ ...config, default_targets: targets });
                }}
              >
                {t}
              </button>
            );
          })}
        </div>
      </div>

      <div className="card">
        <div className="section-header">
          <span className="section-title">About</span>
        </div>
        <div className="card-meta">
          SkillPack v1.0.0
          <br />
          AI Coding Skills Package Manager
          <br />
          Config: {config.version}
          <br />
          Packs: {Object.keys(config.packs).length}
          <br />
          Projects: {Object.keys(config.projects).length}
        </div>
      </div>

      <button className="btn btn-primary" onClick={handleSave}>
        Save Settings
      </button>
    </div>
  );
}

export default Settings;
