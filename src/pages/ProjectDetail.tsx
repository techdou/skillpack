import { useState, useEffect } from "react";
import { ErrorBanner, EmptyState, Pill } from "../components/Feedback";
import {
  packList,
  skillLink,
  skillUnlink,
  skillStatus,
  configGet,
  type PackInfo,
  type SkillLinkInfo,
} from "../lib/api";

interface Props {
  projectPath: string;
  onBack: () => void;
}

function ProjectDetail({ projectPath, onBack }: Props) {
  const [packs, setPacks] = useState<[string, PackInfo][]>([]);
  const [linked, setLinked] = useState<SkillLinkInfo[]>([]);
  const [target, setTarget] = useState("codex");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const projectName = projectPath.split(/[\\/]/).pop() || projectPath;

  const load = async () => {
    try {
      const [packListResult, statusResult] = await Promise.all([
        packList(),
        skillStatus(projectPath).catch(() => []),
      ]);
      setPacks(packListResult);
      setLinked(statusResult);

      // Load default targets from config
      const config = await configGet();
      if (config.default_targets.length > 0) {
        setTarget(config.default_targets[0]);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, [projectPath]);

  const linkedNames = new Set(linked.map((l) => l.skill_name));

  const handleToggle = async (skillName: string, packName: string, isLinked: boolean) => {
    setError(null);
    try {
      if (isLinked) {
        await skillUnlink(projectPath, skillName);
      } else {
        await skillLink(projectPath, skillName, packName, target);
      }
      await load();
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div>
      <button className="back-link" onClick={onBack}>
        <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
          <path d="M11 2L5 8l6 6" stroke="currentColor" strokeWidth="2" fill="none" />
        </svg>
        Projects
      </button>

      <div className="page-title">{projectName}</div>
      <div className="card-meta" style={{ marginBottom: 20 }}>{projectPath}</div>

      <ErrorBanner error={error} />

      <div className="section-header">
        <span className="section-title">Target Toolchain</span>
        <select
          className="select"
          aria-label="Target toolchain"
          value={target}
          onChange={(e) => setTarget(e.target.value)}
        >
          <option value="codex">Codex (.agents/skills)</option>
          <option value="claude">Claude (.claude/skills)</option>
          <option value="gemini">Gemini (.gemini/skills)</option>
        </select>
      </div>

      {linked.length > 0 && (
        <div style={{ marginBottom: 20 }}>
          <div className="section-header">
            <span className="section-title">Linked ({linked.length})</span>
          </div>
          {linked.map((l) => (
            <div className="skill-item" key={l.skill_name}>
              <div>
                <span className="skill-name">{l.skill_name}</span>
                <span className="card-meta" style={{ marginLeft: 8 }}>
                  {l.pack} / {l.target_dir}
                </span>
                <Pill
                  tone={l.link_type === "symlink" ? "ok" : "warn"}
                  title={
                    l.link_type === "symlink"
                      ? "Live link — follows pack updates automatically"
                      : "Copy — snapshot, refreshed on pack update"
                  }
                >
                  {l.link_type === "symlink" ? "Symlink" : "Copy"}
                </Pill>
              </div>
              <button
                className="btn btn-sm btn-danger"
                onClick={() => handleToggle(l.skill_name, l.pack, true)}
              >
                Unlink
              </button>
            </div>
          ))}
        </div>
      )}

      {loading ? (
        <div className="card-meta">Loading packs…</div>
      ) : packs.length === 0 ? (
        <EmptyState
          title="No skill packs installed"
          hint="Go to Packs page to install a skill pack first"
        />
      ) : (
        packs.map(([packName, info]) => (
          <div key={packName} style={{ marginBottom: 16 }}>
            <div className="section-header">
              <span className="section-title">{packName} ({info.skills.length})</span>
            </div>
            {info.skills.map((skill) => {
              const isLinked = linkedNames.has(skill);
              return (
                <div className="skill-item" key={skill}>
                  <span className="skill-name">{skill}</span>
                  <label className="toggle">
                    <input
                      type="checkbox"
                      role="switch"
                      aria-checked={isLinked}
                      aria-label={`Link ${skill} from ${packName}`}
                      checked={isLinked}
                      onChange={() => handleToggle(skill, packName, isLinked)}
                    />
                    <div className="toggle-track" />
                    <div className="toggle-thumb" />
                  </label>
                </div>
              );
            })}
          </div>
        ))
      )}
    </div>
  );
}

export default ProjectDetail;
