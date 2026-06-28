import { useState, useEffect } from "react";
import { ErrorBanner, SuccessBanner } from "../components/Feedback";
import {
  appVersion,
  configGet,
  configUpdateSettings,
  pickDirectory,
  type AppConfig,
} from "../lib/api";

function Settings() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [version, setVersion] = useState("0.1.0");
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState<string | null>(null);

  const load = async () => {
    try {
      const [c, v] = await Promise.all([configGet(), appVersion().catch(() => "0.1.0")]);
      setConfig(c);
      setVersion(v);
    } catch (e) {
      setError(String(e));
    }
  };

  useEffect(() => { load(); }, []);

  const handleSave = async () => {
    if (!config) return;
    setError(null);
    try {
      // Only persist the user-facing fields; packs/projects are preserved by
      // the backend (config_update_settings), avoiding the whole-config
      // overwrite footgun.
      await configUpdateSettings({
        packs_dir: config.packs_dir,
        codex_config_path: config.codex_config_path,
        default_targets: config.default_targets,
      });
      setSaved("Settings saved");
    } catch (e) {
      setError(String(e));
    }
  };

  const choosePacksDir = async () => {
    const selected = await pickDirectory();
    if (selected && config) setConfig({ ...config, packs_dir: selected });
  };

  const chooseCodexConfigDir = async () => {
    // config.toml lives inside a directory; we let the user pick the dir and
    // append the filename using a forward slash so the path works on every
    // platform (the backend normalises via PathBuf::join on read/write).
    const selected = await pickDirectory();
    if (selected && config) {
      const trimmed = selected.replace(/[\\/]+$/, "");
      const path = trimmed.toLowerCase().endsWith(".toml")
        ? trimmed
        : `${trimmed}/config.toml`;
      setConfig({ ...config, codex_config_path: path });
    }
  };

  if (!config) return <div>Loading…</div>;

  return (
    <div>
      <div className="page-title">Settings</div>

      <ErrorBanner error={error} />
      <SuccessBanner message={saved} onDismiss={() => setSaved(null)} />

      <div className="card">
        <div className="section-header">
          <span className="section-title">Storage</span>
        </div>
        <div style={{ marginBottom: 12 }}>
          <label style={{ display: "block", fontSize: 12, color: "var(--text-secondary)", marginBottom: 4 }}>
            Skill Packs Directory
          </label>
          <div className="input-row">
            <input
              className="input"
              value={config.packs_dir}
              onChange={(e) => setConfig({ ...config, packs_dir: e.target.value })}
            />
            <button className="btn" onClick={choosePacksDir}>
              Choose Folder
            </button>
          </div>
        </div>
        <div style={{ marginBottom: 12 }}>
          <label style={{ display: "block", fontSize: 12, color: "var(--text-secondary)", marginBottom: 4 }}>
            Codex config.toml Path
          </label>
          <div className="input-row">
            <input
              className="input"
              value={config.codex_config_path || ""}
              onChange={(e) => setConfig({ ...config, codex_config_path: e.target.value || null })}
            />
            <button className="btn" onClick={chooseCodexConfigDir}>
              Choose Folder
            </button>
          </div>
        </div>
      </div>

      <div className="card">
        <div className="section-header">
          <span className="section-title">Default Toolchains</span>
        </div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          {["codex", "claude", "gemini"].map((t) => {
            const active = config.default_targets.includes(t);
            return (
              <button
                key={t}
                className={`btn btn-sm ${active ? "btn-primary" : ""}`}
                aria-pressed={active}
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
          SkillPack v{version}
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
