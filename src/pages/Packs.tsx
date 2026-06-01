import { useState, useEffect } from "react";
import { packList, packInstall, packRemove, packUpdate, type PackInfo } from "../lib/api";

function Packs() {
  const [packs, setPacks] = useState<[string, PackInfo][]>([]);
  const [url, setUrl] = useState("");
  const [name, setName] = useState("");
  const [skillRoot, setSkillRoot] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [expandedPack, setExpandedPack] = useState<string | null>(null);

  const load = async () => {
    try {
      const list = await packList();
      setPacks(list);
    } catch (e) {
      setError(String(e));
    }
  };

  useEffect(() => { load(); }, []);

  const handleInstall = async () => {
    if (!url || !name) return;
    setLoading(true);
    setError(null);
    try {
      await packInstall(url, name, skillRoot || undefined);
      setUrl("");
      setName("");
      setSkillRoot("");
      await load();
    } catch (e) {
      setError(String(e));
    }
    setLoading(false);
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
      const updated = await packUpdate(packName);
      await load();
      alert(packName ? `Updated ${packName}` : `Updated: ${updated.join(", ")}`);
    } catch (e) {
      setError(String(e));
    }
    setLoading(false);
  };

  return (
    <div>
      <div className="page-title">Skill Packs</div>

      {error && <div style={{ color: "var(--danger)", marginBottom: 12 }}>{error}</div>}

      <div className="card">
        <div className="section-header">
          <span className="section-title">Install New Pack</span>
        </div>
        <div className="input-row">
          <input
            className="input"
            placeholder="Git URL (e.g. https://github.com/...)"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
          />
          <input
            className="input"
            placeholder="Pack name (e.g. ARIS)"
            value={name}
            onChange={(e) => setName(e.target.value)}
            style={{ maxWidth: 180 }}
          />
          <input
            className="input"
            placeholder="Skill root (optional)"
            value={skillRoot}
            onChange={(e) => setSkillRoot(e.target.value)}
            style={{ maxWidth: 200 }}
          />
          <button className="btn btn-primary" onClick={handleInstall} disabled={loading || !url || !name}>
            {loading ? "Installing..." : "Install"}
          </button>
        </div>
      </div>

      {packs.length === 0 ? (
        <div className="empty-state">
          <p>No packs installed yet</p>
          <p style={{ fontSize: 12 }}>Install a skill pack from a Git repository above</p>
        </div>
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
