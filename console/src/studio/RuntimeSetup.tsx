import React, { useState } from "react";

export type RuntimeStatus = {
  phase: string;
  running: boolean;
  ready: boolean;
  publications: { name: string; status: string }[];
  error: string | null;
  phases: { phase: string; duration_ms: number }[];
  elapsed_ms: number;
  current_phase_ms: number;
  session_token: string | null;
  runtime_directory: string | null;
};

const phases: Record<string, string> = {
  locating_package: "Locating accepted runtime",
  copying: "Copying bundle to temporary storage",
  authenticating_kerml: "Authenticating KerML Operational v9",
  authenticating_sysml: "Authenticating SysML Operational v3",
  installing: "Installing authenticated runtime",
  publications_authenticated: "Semantic runtime authenticated",
  opening_repository: "Opening durable repository",
  opening_agentique: "Opening Agentique",
  validating_architecture: "Compiling and validating Agentique architecture",
  validating_agent_fabric: "Compiling and validating Agent Fabric",
  restoring_revision: "Restoring the saved semantic revision",
  ready: "Opening System World",
  setup_required: "Runtime setup needed",
};

export function RuntimeSetup({
  status,
  error,
  onRetry,
  onInstall,
}: {
  status?: RuntimeStatus;
  error: string;
  onRetry: () => Promise<void>;
  onInstall: (bundle: string) => Promise<void>;
}) {
  const [bundle, setBundle] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [installError, setInstallError] = useState("");
  const running = status?.running || status?.ready || submitting;
  async function run(operation: () => Promise<void>) {
    setSubmitting(true);
    setInstallError("");
    try {
      await operation();
    } catch (caught) {
      setInstallError(
        caught instanceof Error ? caught.message : String(caught),
      );
    } finally {
      setSubmitting(false);
    }
  }
  return (
    <main className="studio-startup">
      <div className="studio-startup-mark">◈</div>
      <p className="studio-eyebrow">AGENTIQUE MODELS AGENTIQUE</p>
      <h1>
        {running
          ? "Opening your engineering workspace."
          : "Your semantic runtime starts here."}
      </h1>
      <p>
        Explore a real system. Inspect its meaning. Review and commit changes.
      </p>
      <section
        className="studio-connection-card studio-runtime-card"
        aria-label="Semantic runtime setup"
      >
        <div className="studio-card-label" role="status">
          <span className="studio-status-dot" />
          {status
            ? (phases[status.phase] ?? status.phase)
            : error
              ? "Repository connection needed"
              : "Connecting to Studio"}
        </div>
        <div className="studio-runtime-publications">
          {(
            status?.publications ?? [
              { name: "KerML Operational v9", status: "Not authenticated" },
              { name: "SysML Operational v3", status: "Not authenticated" },
            ]
          ).map((publication) => (
            <div key={publication.name}>
              <span>{publication.name}</span>
              <strong
                className={
                  publication.status === "Authenticated" ? "authenticated" : ""
                }
              >
                {publication.status === "Authenticated" ? "✓ " : "○ "}
                {publication.status}
              </strong>
            </div>
          ))}
        </div>
        {running ? (
          <>
            <p>
              The accepted publications and durable model are being
              authenticated. Large semantic restores can take a few minutes.
            </p>
            <ol className="studio-runtime-phases">
              {status?.phases
                .filter((item) => item.duration_ms > 5)
                .map((item, index) => (
                  <li key={index}>
                    <span>✓ {phases[item.phase] ?? item.phase}</span>
                    <small>{(item.duration_ms / 1000).toFixed(1)}s</small>
                  </li>
                ))}
              <li aria-live="polite">
                <span>{phases[status?.phase ?? ""] ?? "Connecting"}…</span>
                {status && (
                  <small>{Math.floor(status.current_phase_ms / 1000)}s</small>
                )}
              </li>
            </ol>
          </>
        ) : (
          <>
            <p>
              Agentique needs its authenticated KerML and SysML publications
              before it can open a model. Install the accepted runtime bundle
              once, including from an offline copy.
            </p>
            <form
              onSubmit={(event) => {
                event.preventDefault();
                void run(() => onInstall(bundle.trim()));
              }}
            >
              <label htmlFor="runtime-bundle">Local bundle path</label>
              <input
                id="runtime-bundle"
                placeholder="Absolute path to a bundle directory or .agq-runtime file"
                value={bundle}
                onChange={(event) => setBundle(event.target.value)}
              />
              <div className="studio-runtime-actions">
                <button
                  className="studio-primary"
                  type="submit"
                  disabled={!bundle.trim() || !status?.session_token}
                >
                  Use local bundle
                </button>
                <button type="button" onClick={() => void run(onRetry)}>
                  Check installed runtime
                </button>
              </div>
            </form>
            {(installError || status?.error || error) && (
              <details className="studio-runtime-details">
                <summary>Setup details</summary>
                <p role="alert">{installError || status?.error || error}</p>
                {status?.runtime_directory && (
                  <p>
                    Runtime store: <code>{status.runtime_directory}</code>
                  </p>
                )}
              </details>
            )}
          </>
        )}
      </section>
      <p className="studio-runtime-note">
        A fresh repository is created from the checked-in Agentique architecture
        after authentication.
      </p>
      <a className="studio-expert-link" href="/expert">
        Open the legacy expert console ↗
      </a>
    </main>
  );
}
