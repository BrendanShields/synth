import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import "./App.css";

type RuntimeStatus = {
  productName: string;
  appVersion: string;
  runtimeBoundary: string;
  rendererBoundary: string;
  autonomyMode: string;
  planningGate: string;
  workspaceState: string;
  providerState: string;
  eventStreamState: string;
  summary: string;
};

type RuntimeEvent = {
  eventId: string;
  eventType: string;
  status: RuntimeStatus;
};

type RuntimePhase = "loading" | "ready" | "runtime-unavailable";

const RUNTIME_STATUS_EVENT = "synth-runtime-status";

const statusRows: Array<{
  label: string;
  value: keyof RuntimeStatus;
}> = [
  { label: "Runtime boundary", value: "runtimeBoundary" },
  { label: "Renderer boundary", value: "rendererBoundary" },
  { label: "Autonomy mode", value: "autonomyMode" },
  { label: "Planning gate", value: "planningGate" },
  { label: "Workspace", value: "workspaceState" },
  { label: "Provider", value: "providerState" },
  { label: "Event stream", value: "eventStreamState" },
];

function formatLabel(value: string) {
  return value.replace(/_/g, " ");
}

function formatRuntimeError(error: unknown) {
  if (error instanceof Error) {
    return error.message;
  }

  if (typeof error === "string") {
    return error;
  }

  return "Runtime status bridge is unavailable.";
}

function App() {
  const [phase, setPhase] = useState<RuntimePhase>("loading");
  const [runtimeStatus, setRuntimeStatus] = useState<RuntimeStatus | null>(null);
  const [runtimeEvent, setRuntimeEvent] = useState<RuntimeEvent | null>(null);
  const [runtimeError, setRuntimeError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    let unlistenRuntimeStatus: UnlistenFn | undefined;

    async function connectRuntimeBridge() {
      try {
        unlistenRuntimeStatus = await listen<RuntimeEvent>(
          RUNTIME_STATUS_EVENT,
          (event) => {
            if (!active) {
              return;
            }

            setRuntimeEvent(event.payload);
            setRuntimeStatus(event.payload.status);
            setRuntimeError(null);
            setPhase("ready");
          },
        );

        if (!active) {
          unlistenRuntimeStatus();
          return;
        }

        const status = await invoke<RuntimeStatus>("get_runtime_status");

        if (!active) {
          return;
        }

        setRuntimeStatus(status);
        setRuntimeError(null);
        setPhase("ready");

        const announcedEvent = await invoke<RuntimeEvent>(
          "announce_runtime_status",
        );

        if (!active) {
          return;
        }

        setRuntimeEvent(announcedEvent);
        setRuntimeStatus(announcedEvent.status);
        setRuntimeError(null);
        setPhase("ready");
      } catch (error) {
        if (!active) {
          return;
        }

        setRuntimeError(formatRuntimeError(error));
        setPhase("runtime-unavailable");
      }
    }

    connectRuntimeBridge();

    return () => {
      active = false;
      unlistenRuntimeStatus?.();
    };
  }, []);

  const shellStatus = useMemo(() => {
    if (!runtimeStatus) {
      return "Synth · Runtime bridge connecting";
    }

    return `${runtimeStatus.productName} · ${formatLabel(
      runtimeStatus.autonomyMode,
    )} · Planning ${formatLabel(
      runtimeStatus.planningGate,
    )} · Runtime ${formatLabel(runtimeStatus.eventStreamState)}`;
  }, [runtimeStatus]);

  return (
    <main className="synth-shell" aria-label="Synth runtime shell">
      <section className="status-line" aria-live="polite">
        <span>{shellStatus}</span>
        <span className={`phase-pill phase-pill--${phase}`}>
          {formatLabel(phase)}
        </span>
      </section>

      <section className="artifact-card" aria-labelledby="runtime-bridge-title">
        <div className="artifact-kicker">FS-001 · Walking skeleton</div>
        <div className="artifact-heading">
          <div>
            <h1 id="runtime-bridge-title">Runtime event bridge</h1>
            <p>
              The React renderer is now a quiet visual surface for status owned
              by the Rust/Tauri core.
            </p>
          </div>
          <div className="version-mark">
            {runtimeStatus?.appVersion ?? "0.1.0"}
          </div>
        </div>

        {runtimeError ? (
          <div className="runtime-error" role="status">
            <strong>Runtime unavailable</strong>
            <span>{runtimeError}</span>
          </div>
        ) : null}

        {runtimeStatus ? (
          <dl className="status-grid">
            {statusRows.map((row) => (
              <div className="status-item" key={row.value}>
                <dt>{row.label}</dt>
                <dd>{formatLabel(runtimeStatus[row.value])}</dd>
              </div>
            ))}
          </dl>
        ) : (
          <div className="runtime-loading" role="status">
            Waiting for the trusted runtime status snapshot…
          </div>
        )}

        <div className="event-panel">
          <div>
            <span className="eyebrow">Last event</span>
            <strong>{runtimeEvent?.eventType ?? "No runtime event yet"}</strong>
          </div>
          <code>{runtimeEvent?.eventId ?? RUNTIME_STATUS_EVENT}</code>
        </div>

        <p className="summary-copy">
          {runtimeStatus?.summary ??
            "Planning baseline merged. Ready for Phase 1 walking skeleton."}
        </p>
      </section>

      <section className="command-dock" aria-label="Command dock placeholder">
        <span className="dock-prefix">/</span>
        <input
          aria-label="Command dock placeholder"
          disabled
          value="Command dock placeholder — command handling arrives in a later spec"
          readOnly
        />
        <span className="dock-hint">visual only</span>
      </section>
    </main>
  );
}

export default App;
