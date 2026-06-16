import { useEffect, useState } from "react";
import { ErrorBanner, EmptyState } from "../components/Feedback";
import {
  openPath,
  projectList,
  projectSkillRoots,
  projectSkills,
  toolchainSkillRoots,
  toolchainSkills,
  type ProjectInfo,
  type SkillEntry,
  type SkillRootInfo,
} from "../lib/api";

type Scope = "local" | "projects";

function Skills() {
  const [scope, setScope] = useState<Scope>("local");
  const [localRoots, setLocalRoots] = useState<SkillRootInfo[]>([]);
  const [projects, setProjects] = useState<ProjectInfo[]>([]);
  const [projectRoots, setProjectRoots] = useState<SkillRootInfo[]>([]);
  const [selectedToolchain, setSelectedToolchain] = useState("codex");
  const [selectedProject, setSelectedProject] = useState<string | null>(null);
  const [skills, setSkills] = useState<SkillEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const loadLocal = async () => {
    const roots = await toolchainSkillRoots();
    setLocalRoots(roots);
    if (roots.length > 0 && !roots.some((root) => root.key === selectedToolchain)) {
      setSelectedToolchain(roots[0].key);
    }
  };

  const loadProjects = async () => {
    const list = await projectList();
    setProjects(list);
    if (!selectedProject && list.length > 0) {
      setSelectedProject(list[0].path);
    }
  };

  useEffect(() => {
    setError(null);
    Promise.all([loadLocal(), loadProjects()])
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    setError(null);
    if (scope === "local") {
      toolchainSkills(selectedToolchain)
        .then(setSkills)
        .catch((e) => setError(String(e)));
      return;
    }

    if (!selectedProject) {
      setSkills([]);
      setProjectRoots([]);
      return;
    }

    Promise.all([
      projectSkillRoots(selectedProject),
      projectSkills(selectedProject, selectedToolchain),
    ])
      .then(([roots, entries]) => {
        setProjectRoots(roots);
        setSkills(entries);
      })
      .catch((e) => setError(String(e)));
  }, [scope, selectedToolchain, selectedProject]);

  const roots = scope === "local" ? localRoots : projectRoots;

  const openDirectory = async (path: string) => {
    setError(null);
    try {
      await openPath(path);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div>
      <div className="page-title">Skills</div>

      <ErrorBanner error={error} />

      <div className="section-header">
        <div className="segmented">
          <button className={scope === "local" ? "active" : ""} onClick={() => setScope("local")}>
            Local Tools
          </button>
          <button className={scope === "projects" ? "active" : ""} onClick={() => setScope("projects")}>
            Projects
          </button>
        </div>
        {scope === "projects" && (
          <select
            className="select"
            aria-label="Project"
            value={selectedProject || ""}
            onChange={(e) => setSelectedProject(e.target.value || null)}
          >
            {projects.length === 0 && <option value="">No projects</option>}
            {projects.map((project) => (
              <option key={project.path} value={project.path}>
                {project.name}
              </option>
            ))}
          </select>
        )}
      </div>

      {scope === "projects" && selectedProject && (
        <div className="card">
          <div className="card-header">
            <span className="card-title">Project Root</span>
            <button className="btn btn-sm" onClick={() => openDirectory(selectedProject)}>
              Open
            </button>
          </div>
          <div className="card-meta mono-path">{selectedProject}</div>
        </div>
      )}

      <div className="root-grid">
        {roots.map((root) => (
          <button
            className={`root-card ${selectedToolchain === root.key ? "active" : ""}`}
            key={root.key}
            onClick={() => setSelectedToolchain(root.key)}
          >
            <span className="root-card-title">{root.label}</span>
            <span className={`status-dot ${root.exists ? "ok" : ""}`} />
            <span className="root-card-count">{root.skill_count} skills</span>
            <span className="root-card-path">{root.path}</span>
            <span
              className="btn btn-sm"
              onClick={(e) => {
                e.stopPropagation();
                openDirectory(root.path);
              }}
            >
              Open
            </span>
          </button>
        ))}
      </div>

      {loading ? (
        <div className="card-meta">Loading skill directories…</div>
      ) : roots.length === 0 ? (
        <EmptyState
          title="No skill directories found"
          hint="Run the desktop app to scan local and project directories"
        />
      ) : skills.length === 0 ? (
        <EmptyState
          title="No skills in selected directory"
          hint="Missing directories are shown but not created automatically"
        />
      ) : (
        <div>
          <div className="section-header">
            <span className="section-title">Skills ({skills.length})</span>
          </div>
          {skills.map((skill) => (
            <div className="skill-item skill-item-detail" key={skill.path}>
              <div>
                <div className="skill-name">{skill.name}</div>
                <div className="skill-desc wide">{skill.description}</div>
                <div className="card-meta mono-path">{skill.path}</div>
              </div>
              <button className="btn btn-sm" onClick={() => openDirectory(skill.path)}>
                Open
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export default Skills;
