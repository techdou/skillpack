import { useEffect, useState } from "react";
import { ErrorBanner, EmptyState, SuccessBanner } from "../components/Feedback";
import {
  featuredList,
  featuredRefresh,
  packInstall,
  packInstallLocal,
  packList,
  packOpen,
  packRemove,
  packUpdate,
  pickDirectory,
  type FeaturedPack,
  type PackInfo,
} from "../lib/api";

function Packs() {
  const [packs, setPacks] = useState<[string, PackInfo][]>([]);
  const [featured, setFeatured] = useState<FeaturedPack[]>([]);
  const [installMode, setInstallMode] = useState<"git" | "local">("git");
  const [url, setUrl] = useState("");
  const [localDir, setLocalDir] = useState("");
  const [name, setName] = useState("");
  const [skillRoot, setSkillRoot] = useState("");
  const [loading, setLoading] = useState(false);
  const [listLoading, setListLoading] = useState(true);
  const [featuredLoading, setFeaturedLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [expandedPack, setExpandedPack] = useState<string | null>(null);
  /** Pack names currently being installed from the Featured grid. */
  const [installingFeatured, setInstallingFeatured] = useState<Set<string>>(new Set());

  const loadPacks = async () => {
    try {
      const list = await packList();
      setPacks(list);
    } catch (e) {
      setError(String(e));
    } finally {
      setListLoading(false);
    }
  };

  const loadFeatured = async () => {
    try {
      const list = await featuredList();
      setFeatured(list);
    } catch {
      // featured_list already falls back to an embedded catalog on the backend;
      // a JS-level failure here just leaves the grid hidden rather than crashing.
    } finally {
      setFeaturedLoading(false);
    }
  };

  useEffect(() => {
    loadPacks();
    loadFeatured();
  }, []);

  const installedNames = new Set(packs.map(([n]) => n));

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
      await loadPacks();
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
      await loadPacks();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleUpdate = async (packName?: string) => {
    setLoading(true);
    setError(null);
    try {
      const report = await packUpdate(packName);
      await loadPacks();
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

  /** Install a featured pack by reusing the standard pack_install path. */
  const handleInstallFeatured = async (fp: FeaturedPack) => {
    setError(null);
    setSuccess(null);
    setInstallingFeatured((prev) => new Set(prev).add(fp.name));
    try {
      await packInstall(fp.source, fp.name, fp.skill_root || undefined);
      await loadPacks();
      setSuccess(`Installed "${fp.name}".`);
    } catch (e) {
      setError(String(e));
    } finally {
      setInstallingFeatured((prev) => {
        const next = new Set(prev);
        next.delete(fp.name);
        return next;
      });
    }
  };

  /** Force-refresh the registry and swap in the new featured list. */
  const handleRefreshFeatured = async () => {
    setRefreshing(true);
    setError(null);
    try {
      const list = await featuredRefresh();
      setFeatured(list);
      setSuccess("Featured catalog refreshed.");
    } catch (e) {
      setError(String(e));
    } finally {
      setRefreshing(false);
    }
  };

  return (
    <div>
      <div className="page-title">Skill Packs</div>

      <ErrorBanner error={error} />
      <SuccessBanner message={success} onDismiss={() => setSuccess(null)} />

      {/* Featured Packs — discovery layer, always present (backend falls back
          to an embedded catalog, so this grid is never blank). */}
      {!featuredLoading && featured.length > 0 && (
        <div className="featured-section">
          <div className="section-header">
            <div>
              <span className="section-title">★ Featured Packs</span>
              <div className="featured-section-hint">
                Curated packs — install with one click, no URL needed.
              </div>
            </div>
            <button
              className="btn btn-sm"
              onClick={handleRefreshFeatured}
              disabled={refreshing}
            >
              {refreshing ? "Refreshing…" : "Refresh"}
            </button>
          </div>
          <div className="featured-grid">
            {featured.map((fp) => {
              const isInstalled = installedNames.has(fp.name);
              const isInstalling = installingFeatured.has(fp.name);
              return (
                <div className="featured-card" key={fp.id}>
                  <div className="featured-card-head">
                    <span className="featured-card-name">{fp.name}</span>
                    {fp.verified && (
                      <span className="pill pill-ok" title="Verified author">✓ Verified</span>
                    )}
                  </div>
                  <div className="featured-card-desc">{fp.description}</div>
                  <div className="featured-card-meta">
                    {fp.category && <span>{fp.category}</span>}
                    {fp.license && <span>· {fp.license}</span>}
                    {fp.author && <span>· {fp.author}</span>}
                  </div>
                  <button
                    className={`btn btn-sm ${isInstalled ? "" : "btn-primary"}`}
                    onClick={() =>
                      !isInstalled && !isInstalling && handleInstallFeatured(fp)
                    }
                    disabled={isInstalled || isInstalling}
                  >
                    {isInstalled ? "✓ Installed" : isInstalling ? "Installing…" : "Install"}
                  </button>
                </div>
              );
            })}
          </div>
        </div>
      )}

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
          hint="Install a featured pack above, or add one from a Git URL or local directory."
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
