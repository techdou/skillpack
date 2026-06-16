import { useEffect, useState } from "react";
import { ErrorBanner, EmptyState, SuccessBanner } from "../components/Feedback";
import {
  packInstall,
  packInstallLocal,
  packList,
  packOpen,
  packRemove,
  packUpdate,
  pickDirectory,
  type PackInfo,
} from "../lib/api";

function Packs() {
  const [packs, setPacks] = useState<[string, PackInfo][]>([]);
  const [installMode, setInstallMode] = useState<"git" | "local">("git");
  const [url, setUrl] = useState("");
  const [localDir, setLocalDir] = useState("");
  const [name, setName] = useState("");
  const [skillRoot, setSkillRoot] = useState("");
  const [loading, setLoading] = useState(false);
  const [listLoading, setListLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [expandedPack, setExpandedPack] = useState<string | null>(null);

  const load = async () => {
    try {
      const list = await packList();
      setPacks(list);
    } catch (e) {
      setError(String(e));
    } finally {
      setListLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  const chooseLocalPack = async () => {
    setError(null);
    try {
      const selected = await pickDirectory();
      if (!selected) return;
      setLocalDir(selected);
      if (!name) {
        setName(selected.split(/[\\/]/).filter(Boolean).pop() || "");
      }
    } catch (e) {
      setError(String(e));
    }
  };

  const chooseSkillRoot = async () => {
    setError(null);
    try {
      const selected = await pickDirectory();
      if (selected) setSkillRoot(selected);
    } catch (e) {
      setError(String(e));
    }
  };

  const handleInstall = async () => {
    if (!name || (installMode === "git" ? !url : !localDir)) return;
    setLoading(true);
    setError(null);
    try {
      if (installMode === "git") {
        await packInstall(url, name, skillRoot || undefined);
      } else {
        await packInstallLocal(localDir, name, skillRoot || undefined);
      }
      setUrl("");
      setLocalDir("");
      setName("");
      setSkillRoot("");
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleRemove = async (packName: string) => {
    if (!confirm(`Remove pack "${packName}" and all its project links?`)) return;
    try {
      await packRemove(packName);
      await load();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleUpdate = async (packName?: string) => {
    setLoading(true);
    setError(null);
    try {
      const report = await packUpdate(packName);
      await load();
      const parts: string[] = [];
      if (report.updated.length) parts.push(`Updated: ${report.updated.join(", ")}`);
      if (report.failed.length) {
        parts.push(
          `Failed: ${report.failed.map((f) => `${f.pack} (${f.error})`).join(", ")}`,
        );
      }
      setSuccess(parts.join(" — ") || "Nothing to update.");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const openPack = async (packName: string) => {
    setError(null);
    try {
      await packOpen(packName);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div>
      <div className="page-title">Skill Packs</div>

      <ErrorBanner error={error} />
      <SuccessBanner message={success} onDismiss={() => setSuccess(null)} />

      <div className="card">
        <div className="section-header">
          <span className="section-title">Install New Pack</span>
          <div className="segmented">
            <button
              className={installMode === "git" ? "active" : ""}
              onClick={() => setInstallMode("git")}
            >
              Git URL
            </button>
            <button
              className={installMode === "local" ? "active" : ""}
              onClick={() => setInstallMode("local")}
            >
              Local Directory
            </button>
          </div>
        </div>
        <div className="input-row">
          {installMode === "git" ? (
            <input
              className="input"
              placeholder="Git URL (e.g. https://github.com/...)"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
            />
          ) : (
            <>
              <input
                className="input"
                placeholder="Local pack directory"
                value={localDir}
                readOnly
              />
              <button className="btn" onClick={chooseLocalPack}>
                Choose Folder
              </button>
            </>
          )}
          <input
            className="input"
            placeholder="Pack name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            style={{ maxWidth: 180 }}
          />
          <input
            className="input"
            placeholder={installMode === "git" ? "Skill root relative path" : "Skill root directory"}
            value={skillRoot}
            onChange={(e) => setSkillRoot(e.target.value)}
            style={{ maxWidth: 220 }}
          />
          {installMode === "local" && (
            <button className="btn" onClick={chooseSkillRoot} disabled={!localDir}>
              Skill Root
            </button>
          )}
          <button
            className="btn btn-primary"
            onClick={handleInstall}
            disabled={loading || !name || (installMode === "git" ? !url : !localDir)}
          >
            {loading ? "Installing..." : "Install"}
          </button>
        </div>
      </div>

      {listLoading ? (
        <div className="card placeholder-card" aria-hidden="true">
          <div className="card-header"><span className="card-title">…</span></div>
          <div className="card-meta">Loading packs…</div>
        </div>
      ) : packs.length === 0 ? (
        <EmptyState
          title="No packs installed yet"
          hint="Install a skill pack from Git or a local directory"
        />
      ) : (
        <>
          <div className="section-header">
            <span className="section-title">Installed ({packs.length})</span>
            <button className="btn btn-sm" onClick={() => handleUpdate()}>
              Update All
            </button>
          </div>
          {packs.map(([packName, info]) => (
            <div className="card" key={packName}>
              <div className="card-header">
                <div>
                  <span className="card-title">{packName}</span>
                  <span className="badge badge-count" style={{ marginLeft: 8 }}>
                    {info.skills.length} skills
                  </span>
                </div>
                <div style={{ display: "flex", gap: 6 }}>
                  <button className="btn btn-sm" onClick={() => openPack(packName)}>
                    Open
                  </button>
                  <button className="btn btn-sm" onClick={() => handleUpdate(packName)}>
                    Update
                  </button>
                  <button className="btn btn-sm btn-danger" onClick={() => handleRemove(packName)}>
                    Remove
                  </button>
                  <button
                    className="btn btn-sm"
                    onClick={() => setExpandedPack(expandedPack === packName ? null : packName)}
                  >
                    {expandedPack === packName ? "Collapse" : "Skills"}
                  </button>
                </div>
              </div>
              <div className="card-meta">
                {info.source}
                {info.skill_root && <span style={{ marginLeft: 8 }}>root: {info.skill_root}</span>}
                <span style={{ marginLeft: 8 }}>installed: {new Date(info.installed_at).toLocaleDateString()}</span>
              </div>
              {expandedPack === packName && (
                <div style={{ marginTop: 12 }}>
                  {info.skills.map((s) => (
                    <div key={s} className="skill-item">
                      <span className="skill-name">{s}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          ))}
        </>
      )}
    </div>
  );
}

export default Packs;
