import { useState } from "react";
import { ErrorBoundary } from "./components/ErrorBoundary";
import Packs from "./pages/Packs";
import Projects from "./pages/Projects";
import ProjectDetail from "./pages/ProjectDetail";
import Plugins from "./pages/Plugins";
import Skills from "./pages/Skills";
import Settings from "./pages/Settings";

type Page = "packs" | "projects" | "project-detail" | "skills" | "plugins" | "settings";

interface AppState {
  page: Page;
  selectedProject: string | null;
}

const icons = {
  packs: (
    <svg viewBox="0 0 16 16" fill="currentColor">
      <path d="M1 3.5a.5.5 0 0 1 .5-.5h13a.5.5 0 0 1 .5.5v1a.5.5 0 0 1-.5.5h-13a.5.5 0 0 1-.5-.5v-1zM1 7.5a.5.5 0 0 1 .5-.5h13a.5.5 0 0 1 .5.5v1a.5.5 0 0 1-.5.5h-13a.5.5 0 0 1-.5-.5v-1zM1 11.5a.5.5 0 0 1 .5-.5h13a.5.5 0 0 1 .5.5v1a.5.5 0 0 1-.5.5h-13a.5.5 0 0 1-.5-.5v-1z" />
    </svg>
  ),
  projects: (
    <svg viewBox="0 0 16 16" fill="currentColor">
      <path d="M1 3.5A1.5 1.5 0 0 1 2.5 2h2.764c.958 0 1.76.56 2.311 1.184C7.985 3.648 8.48 4 9 4h4.5A1.5 1.5 0 0 1 15 5.5v7a1.5 1.5 0 0 1-1.5 1.5h-11A1.5 1.5 0 0 1 1 12.5v-9z" />
    </svg>
  ),
  plugins: (
    <svg viewBox="0 0 16 16" fill="currentColor">
      <path d="M1 8a7 7 0 1 1 2.878 5.674l-.726.727a1 1 0 0 1-1.414-1.414l.727-.727A7 7 0 0 1 1 8zm7-4a.75.75 0 0 1 .75.75v2.5h2.5a.75.75 0 0 1 0 1.5h-2.5v2.5a.75.75 0 0 1-1.5 0v-2.5h-2.5a.75.75 0 0 1 0-1.5h2.5v-2.5A.75.75 0 0 1 8 4z" />
    </svg>
  ),
  skills: (
    <svg viewBox="0 0 16 16" fill="currentColor">
      <path d="M3 2.5A1.5 1.5 0 0 1 4.5 1h7A1.5 1.5 0 0 1 13 2.5v11A1.5 1.5 0 0 1 11.5 15h-7A1.5 1.5 0 0 1 3 13.5v-11zm2.25 3a.75.75 0 0 0 0 1.5h5.5a.75.75 0 0 0 0-1.5h-5.5zm0 3a.75.75 0 0 0 0 1.5h5.5a.75.75 0 0 0 0-1.5h-5.5z" />
    </svg>
  ),
  settings: (
    <svg viewBox="0 0 16 16" fill="currentColor">
      <path d="M8 4.754a3.246 3.246 0 1 0 0 6.492 3.246 3.246 0 0 0 0-6.492zM5.754 8a2.246 2.246 0 1 1 4.492 0 2.246 2.246 0 0 1-4.492 0z" />
      <path d="M9.796 1.343c-.527-1.79-3.065-1.79-3.592 0l-.094.319a.873.873 0 0 1-1.255.52l-.292-.16c-1.283-.698-2.686.705-1.987 1.987l.169.311c.446.82.023 1.841-.872 2.105l-.34.1c-1.4.413-1.4 2.397 0 2.81l.34.1a1.464 1.464 0 0 1 .872 2.105l-.17.31c-.698 1.283.705 2.686 1.987 1.987l.311-.169a1.464 1.464 0 0 1 2.105.872l.1.34c.413 1.4 2.397 1.4 2.81 0l.1-.34a1.464 1.464 0 0 1 2.105-.872l.31.17c1.283.698 2.686-.705 1.987-1.987l-.169-.311a1.464 1.464 0 0 1 .872-2.105l.34-.1c1.4-.413 1.4-2.397 0-2.81l-.34-.1a1.464 1.464 0 0 1-.872-2.105l.17-.31c.698-1.283-.705-2.686-1.987-1.987l-.311.169a1.464 1.464 0 0 1-2.105-.872l-.1-.34z" />
    </svg>
  ),
};

function App() {
  const [state, setState] = useState<AppState>({ page: "packs", selectedProject: null });

  const navigate = (page: Page, selectedProject?: string) => {
    setState({ page, selectedProject: selectedProject ?? null });
  };

  const navItems: { key: Page; label: string }[] = [
    { key: "packs", label: "Packs" },
    { key: "projects", label: "Projects" },
    { key: "skills", label: "Skills" },
    { key: "plugins", label: "Plugins" },
    { key: "settings", label: "Settings" },
  ];

  return (
    <>
      <nav className="sidebar">
        <div className="sidebar-logo">
          <img src="/logo.svg" alt="" aria-hidden="true" />
          Skill<span>Pack</span>
        </div>
        <div className="sidebar-nav">
          {navItems.map(({ key, label }) => (
            <button
              key={key}
              className={`sidebar-item ${state.page === key ? "active" : ""}`}
              onClick={() => navigate(key)}
            >
              {icons[key as keyof typeof icons]}
              {label}
            </button>
          ))}
        </div>
      </nav>

      <main className="main">
        <ErrorBoundary>
          {state.page === "packs" && <Packs />}
          {state.page === "projects" && (
            <Projects onSelectProject={(path) => navigate("project-detail", path)} />
          )}
          {state.page === "project-detail" && state.selectedProject && (
            <ProjectDetail
              projectPath={state.selectedProject}
              onBack={() => navigate("projects")}
            />
          )}
          {state.page === "skills" && <Skills />}
          {state.page === "plugins" && <Plugins />}
          {state.page === "settings" && <Settings />}
        </ErrorBoundary>
      </main>
    </>
  );
}

export default App;
