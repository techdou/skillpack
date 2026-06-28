import type { PluginEntry } from "../../lib/api";

/**
 * Expanded detail panel for one plugin: every manifest field we have, plus the
 * list of bundled skills and the installed path. Rendered inline below the
 * grid when a plugin is selected.
 */
export function PluginDetail({
  plugin,
  onClose,
}: {
  plugin: PluginEntry;
  onClose: () => void;
}) {
  return (
    <div className="plugin-detail">
      <div className="plugin-detail-head">
        <span className="plugin-detail-title">{plugin.name}</span>
        <button className="btn btn-sm" onClick={onClose} aria-label="Close detail">
          Close
        </button>
      </div>

      <dl className="kv">
        <Row label="Key" value={plugin.key} />
        <Row label="Source" value={plugin.source || "—"} />
        <Row label="Enabled" value={plugin.enabled ? "yes" : "no"} />
        <Row label="Installed" value={plugin.installed ? "yes" : "no"} />
        {plugin.version ? <Row label="Version" value={plugin.version} /> : null}
        {plugin.category ? <Row label="Category" value={plugin.category} /> : null}
        {plugin.author_name ? <Row label="Author" value={plugin.author_name} /> : null}
        {plugin.installed_path ? <Row label="Path" value={plugin.installed_path} mono /> : null}
      </dl>

      {plugin.description ? (
        <div className="plugin-detail-section">
          <div className="plugin-detail-section-title">Description</div>
          <div className="plugin-detail-text">{plugin.description}</div>
        </div>
      ) : null}

      {plugin.capabilities.length > 0 ? (
        <div className="plugin-detail-section">
          <div className="plugin-detail-section-title">Capabilities</div>
          <div className="plugin-detail-chips">
            {plugin.capabilities.map((c) => (
              <span key={c} className="chip">
                {c}
              </span>
            ))}
          </div>
        </div>
      ) : null}

      {plugin.bundled_skills.length > 0 ? (
        <div className="plugin-detail-section">
          <div className="plugin-detail-section-title">
            Bundled skills ({plugin.bundled_skills.length})
          </div>
          <ul className="plugin-detail-skills">
            {plugin.bundled_skills.map((s) => (
              <li key={s}>{s}</li>
            ))}
          </ul>
        </div>
      ) : null}
    </div>
  );
}

function Row({
  label,
  value,
  mono,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="kv-row">
      <dt>{label}</dt>
      <dd className={mono ? "mono" : undefined}>{value}</dd>
    </div>
  );
}
