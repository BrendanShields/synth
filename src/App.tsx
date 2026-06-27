import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { formatLabel, formatRuntimeError, lineSpreadOffset } from "./runtime";
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

  useEffect(() => {
    const scroller = document.querySelector<HTMLElement>(".doc-scroll");
    if (!scroller) {
      return;
    }

    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      return;
    }

    const SELECTOR =
      ".doc-head h1, .doc-lede, .doc-prose, .doc-section h2, .doc-status__row, .doc-quote, .doc-error";

    let frame = 0;

    function update() {
      frame = 0;
      const vh = window.innerHeight;
      scroller
        ?.querySelectorAll<HTMLElement>(SELECTOR)
        .forEach((line) => {
          const rect = line.getBoundingClientRect();
          const center = rect.top + rect.height / 2;
          const offset = lineSpreadOffset(center, vh);
          line.style.transform = `translateY(${offset.toFixed(1)}px)`;
        });
    }

    function onScroll() {
      if (!frame) {
        frame = requestAnimationFrame(update);
      }
    }

    update();
    scroller.addEventListener("scroll", onScroll, { passive: true });
    window.addEventListener("resize", onScroll);

    return () => {
      scroller.removeEventListener("scroll", onScroll);
      window.removeEventListener("resize", onScroll);
      if (frame) {
        cancelAnimationFrame(frame);
      }
    };
  }, [runtimeStatus, runtimeEvent, runtimeError]);

  return (
    <main
      className="synth-shell"
      data-phase={phase}
      aria-label="Synth runtime shell"
    >
      <div className="doc-scroll">
      <aside className="doc-nav" aria-label="Contents">
        <p className="doc-nav__title">Contents</p>
        <nav>
          <a className="doc-nav__link is-active" href="#summary">
            Summary
          </a>
          <a className="doc-nav__link" href="#runtime-status">
            Runtime status
          </a>
          <a className="doc-nav__link" href="#event-stream">
            Event stream
          </a>
          <a className="doc-nav__link" href="#phase">
            Phase
          </a>
        </nav>
      </aside>

      <article className="doc-body">
        <header className="doc-head" id="summary">
          <h1>Synth</h1>
          <p className="doc-lede">
            The runtime event bridge. The Rust/Tauri core owns truth; the React
            renderer is a quiet visual surface for it.
          </p>
        </header>

        <p className="doc-prose">
          {runtimeStatus?.summary ??
            "Planning baseline merged. Ready for Phase 1 walking skeleton."}
        </p>

        {runtimeError ? (
          <div className="doc-error" role="status">
            <strong>Runtime unavailable</strong>
            <span>{runtimeError}</span>
          </div>
        ) : null}

        <section className="doc-section" id="runtime-status">
          <h2>Runtime status</h2>
          {runtimeStatus ? (
            <dl className="doc-status">
              {statusRows.map((row) => (
                <div className="doc-status__row" key={row.value}>
                  <dt>{row.label}</dt>
                  <dd>{formatLabel(runtimeStatus[row.value])}</dd>
                </div>
              ))}
            </dl>
          ) : (
            <p className="doc-prose doc-prose--muted" role="status">
              Waiting for the trusted runtime status snapshot…
            </p>
          )}
        </section>

        <section className="doc-section" id="event-stream">
          <h2>Event stream</h2>
          <blockquote className="doc-quote">
            {runtimeEvent?.eventType ?? "No runtime event yet"}
            <cite>{runtimeEvent?.eventId ?? RUNTIME_STATUS_EVENT}</cite>
          </blockquote>
        </section>

        <section className="doc-section" id="phase">
          <h2>Phase</h2>
          <p className="doc-prose">
            {runtimeStatus
              ? `${runtimeStatus.productName} runtime is ${formatLabel(
                  runtimeStatus.eventStreamState,
                )}. Autonomy ${formatLabel(
                  runtimeStatus.autonomyMode,
                )}, planning ${formatLabel(runtimeStatus.planningGate)}.`
              : "Synth · runtime bridge connecting."}
          </p>
        </section>
      </article>
      </div>

      <div className="doc-dock">
        <div className="doc-input" aria-label="Command dock placeholder">
          <span className="doc-input__prefix">&gt;</span>
          <input
            aria-label="Ask this document"
            placeholder="Ask this document…"
            disabled
            readOnly
          />
          <span className="doc-input__caret" aria-hidden="true" />
        </div>
      </div>

      <footer className="doc-foot">
        <span>
          <kbd>⌘F</kbd> find
        </span>
        <span>
          <kbd>e</kbd> edit
        </span>
        <span>
          <kbd>esc</kbd> back
        </span>
      </footer>
    </main>
  );
}

export default App;
