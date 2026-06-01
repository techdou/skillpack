import { useState, useEffect } from "react";
import { projectList, projectAdd, projectRemove, type ProjectInfo } from "../lib/api";

interface Props {
  onSelectProject: (path: string) => void;
}

function Projects({ onSelectProject }: Props) {
  const [projects, setProjects] = useState<ProjectInfo[]>([]);
  const [newPath, setNewPath] = useState("");
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    try {
      const list = await projectList();
      setProjects(list);
    } catch (e) {
      setError(String(e));
    }
  };

  useEffect(() => { load(); }, []);

  const handleAdd = async () => {
    if (!newPath) return;
    setError(null);
    try {
      await projectAdd(newPath);
      setNewPath("");
      await load();
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

      {error && <div style={{ color: "var(--danger)", marginBottom: 12 }}>{error}</div>}

      <div className="card">
        <div className="section-header">
          <span className="section-title">Add Project</span>
        </div>
        <div className="input-row">
          <input
            className="input"
            placeholder="Project path (e.g. E:\my-paper-project)"
            value={newPath}
            onChange={(e) => setNewPath(e.target.value)}
          />
          <button className="btn btn-primary" onClick={handleAdd} disabled={!newPath}>
            Add
          </button>
        </div>
      </div>

      {projects.length === 0 ? (
        <div className="empty-state">
          <p>No projects registered</p>
          <p style={{ fontSize: 12 }}>Add a project directory to start linking skills</p>
        </div>
      ) : (
        projects.map((proj) => (
          <div
            className="card"
            key={proj.path}
            style={{ cursor: "pointer" }}
            onClick={() => onSelectProject(proj.path)}
          >
            <div className="card-header">
              <span className="card-title">{proj.name}</span>
              <div style={{ display: "flex", gap: 6 }}>
                <span className="badge badge-count">{proj.linked_skills_count} linked</span>
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
