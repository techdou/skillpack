import { useEffect, useState } from "react";
import { ErrorBanner, EmptyState } from "../components/Feedback";
import {
  openPath,
  pickDirectory,
  projectAdd,
  projectList,
  projectRemove,
  type ProjectInfo,
} from "../lib/api";

interface Props {
  onSelectProject: (path: string) => void;
}

function Projects({ onSelectProject }: Props) {
  const [projects, setProjects] = useState<ProjectInfo[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const load = async () => {
    try {
      const list = await projectList();
      setProjects(list);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  const handleAdd = async () => {
    setError(null);
    try {
      const selected = await pickDirectory();
      if (!selected) return;
      await projectAdd(selected);
      await load();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleOpen = async (path: string) => {
    setError(null);
    try {
      await openPath(path);
    } catch (e) {
      setError(String(e));
    }
  };

  const handleRemove = async (path: string) => {
    if (!confirm("Remove project and unlink all skills?")) return;
    try {
      await projectRemove(path);
      await load();
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div>
      <div className="page-title">Projects</div>

      <ErrorBanner error={error} />

      <div className="card">
        <div className="section-header">
          <span className="section-title">Add Project</span>
          <button className="btn btn-primary" onClick={handleAdd}>
            Choose Folder
          </button>
        </div>
        <div className="card-meta">
          Select a project folder with the system file manager. SkillPack will scan toolchain skill
          folders under that project.
        </div>
      </div>

      {loading ? (
        <div className="card-meta">Loading projects…</div>
      ) : projects.length === 0 ? (
        <EmptyState
          title="No projects registered"
          hint="Choose a project directory to start linking skills"
        />
      ) : (
        projects.map((proj) => (
          <div
            className="card clickable"
            key={proj.path}
            role="button"
            tabIndex={0}
            onClick={() => onSelectProject(proj.path)}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                onSelectProject(proj.path);
              }
            }}
          >
            <div className="card-header">
              <span className="card-title">{proj.name}</span>
              <div style={{ display: "flex", gap: 6 }}>
                <span className="badge badge-count">{proj.linked_skills_count} linked</span>
                <button
                  className="btn btn-sm"
                  onClick={(e) => { e.stopPropagation(); handleOpen(proj.path); }}
                >
                  Open
                </button>
                <button
                  className="btn btn-sm btn-danger"
                  onClick={(e) => { e.stopPropagation(); handleRemove(proj.path); }}
                >
                  Remove
                </button>
              </div>
            </div>
            <div className="card-meta">
              {proj.path}
              <span style={{ marginLeft: 8 }}>targets: {proj.targets.join(", ")}</span>
            </div>
          </div>
        ))
      )}
    </div>
  );
}

export default Projects;
