import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  message: string;
}

/**
 * Global render-error boundary. Without it, a render-time exception in any page
 * would white-screen the whole app. Caught errors show a friendly fallback with
 * a "Reload" action instead.
 */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false, message: "" };

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, message: error?.message ?? String(error) };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    // eslint-disable-next-line no-console
    console.error("Uncaught render error:", error, info);
  }

  handleReload = () => {
    this.setState({ hasError: false, message: "" });
    // Best-effort: reload the webview document.
    if (typeof location !== "undefined" && location.reload) {
      location.reload();
    }
  };

  render() {
    if (!this.state.hasError) return this.props.children;
    return (
      <div className="crash-screen">
        <h2>Something went wrong</h2>
        <p className="crash-message">{this.state.message}</p>
        <button className="btn btn-primary" onClick={this.handleReload}>
          Reload
        </button>
      </div>
    );
  }
}
