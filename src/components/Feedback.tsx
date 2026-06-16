import { useEffect, useState, type ReactNode } from "react";

/** Inline red error banner. Replaces ad-hoc `{error && <div style={{color}}>` blocks. */
export function ErrorBanner({ error }: { error: string | null }) {
  if (!error) return null;
  return <div className="banner banner-error">{error}</div>;
}

/**
 * Auto-dismissing success banner. Used by Plugins/Settings to show a transient
 * confirmation that disappears after `ms` (default 2s), giving consistent UX.
 */
export function SuccessBanner({
  message,
  ms = 2000,
  onDismiss,
}: {
  message: string | null;
  ms?: number;
  onDismiss?: () => void;
}) {
  const [visible, setVisible] = useState(!!message);

  useEffect(() => {
    setVisible(!!message);
    if (!message) return;
    const id = setTimeout(() => {
      setVisible(false);
      onDismiss?.();
    }, ms);
    return () => clearTimeout(id);
  }, [message, ms, onDismiss]);

  if (!message || !visible) return null;
  return <div className="banner banner-success">{message}</div>;
}

/** Standard empty state with a title and optional hint. */
export function EmptyState({ title, hint }: { title: string; hint?: ReactNode }) {
  return (
    <div className="empty-state">
      <div className="empty-state-title">{title}</div>
      {hint ? <div className="empty-state-hint">{hint}</div> : null}
    </div>
  );
}

/** Small colored pill. Used for link-type badges (symlink / copy). */
export function Pill({
  tone,
  children,
  title,
}: {
  tone: "ok" | "warn";
  children: ReactNode;
  title?: string;
}) {
  return (
    <span className={`pill pill-${tone}`} title={title}>
      {children}
    </span>
  );
}
