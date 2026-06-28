import { useEffect, useRef, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  appendCommandRouteLogEntry,
  appendSessionEvent,
  formatActiveArtifact,
  formatApprovalState,
  formatCommandError,
  formatCommandArgument,
  formatLabel,
  formatModelError,
  formatProviderState,
  formatRuntimeError,
  formatSessionEvent,
  formatSpecDetailError,
  formatSpecsIndexError,
  formatSpecsIndexSource,
  handledAskQuestion,
  handledSpecDetailId,
  lineSpreadOffset,
  routeTargetElementId,
  isHandledRoute,
  shouldSubmitCommandInput,
  type CommandRoute,
  type ProviderStatus,
  type SessionEvent,
  type SessionEventKind,
  type SpecsIndex,
  type StaticSpecDetail,
} from "./runtime";
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
  const [commandValue, setCommandValue] = useState("");
  const [commandRoutes, setCommandRoutes] = useState<CommandRoute[]>([]);
  const [commandError, setCommandError] = useState<string | null>(null);
  const [specsIndex, setSpecsIndex] = useState<SpecsIndex | null>(null);
  const [specsIndexError, setSpecsIndexError] = useState<string | null>(null);
  const [specDetail, setSpecDetail] = useState<StaticSpecDetail | null>(null);
  const [specDetailError, setSpecDetailError] = useState<string | null>(null);
  const [providerStatus, setProviderStatus] = useState<ProviderStatus | null>(
    null,
  );
  const [answerPrompt, setAnswerPrompt] = useState<string | null>(null);
  const [answerText, setAnswerText] = useState("");
  const [answerPending, setAnswerPending] = useState(false);
  const [answerError, setAnswerError] = useState<string | null>(null);
  const [answerGrounding, setAnswerGrounding] = useState<string | null>(null);
  const requestCounter = useRef(0);
  const currentRequest = useRef(0);
  const currentAsk = useRef<{ prompt: string; grounding: string | null } | null>(
    null,
  );
  const [sessionEvents, setSessionEvents] = useState<SessionEvent[]>([]);
  const eventCounter = useRef(0);

  function recordEvent(
    kind: SessionEventKind,
    label: string,
    detail: string,
  ) {
    const id = (eventCounter.current += 1);
    setSessionEvents((events) =>
      appendSessionEvent(events, { id, kind, label, detail }),
    );
  }

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
    let active = true;

    async function loadSpecsIndex() {
      try {
        const index = await invoke<SpecsIndex>("list_specs_index");

        if (!active) {
          return;
        }

        setSpecsIndex(index);
        setSpecsIndexError(null);
      } catch (error) {
        if (!active) {
          return;
        }

        setSpecsIndexError(formatSpecsIndexError(error));
      }
    }

    loadSpecsIndex();

    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    let active = true;

    invoke<ProviderStatus>("get_provider_status")
      .then((status) => {
        if (active) {
          setProviderStatus(status);
        }
      })
      .catch(() => {
        if (active) {
          setProviderStatus(null);
        }
      });

    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    const unlistens: UnlistenFn[] = [];
    const isCurrent = (requestId: number) =>
      requestId === currentRequest.current;

    listen<{ requestId: number; token: string }>(
      "synth-answer-chunk",
      (event) => {
        if (isCurrent(event.payload.requestId)) {
          setAnswerText((text) => text + event.payload.token);
        }
      },
    ).then((unlisten) => unlistens.push(unlisten));

    listen<{ requestId: number; answer: string }>(
      "synth-answer-done",
      (event) => {
        if (isCurrent(event.payload.requestId)) {
          setAnswerText(event.payload.answer);
          setAnswerPending(false);
          const grounding = currentAsk.current?.grounding ?? null;
          recordEvent(
            "answer",
            grounding ? `answer (${grounding})` : "answer",
            event.payload.answer.slice(0, 80),
          );
        }
      },
    ).then((unlisten) => unlistens.push(unlisten));

    listen<{ requestId: number; message: string }>(
      "synth-answer-error",
      (event) => {
        if (isCurrent(event.payload.requestId)) {
          setAnswerError(event.payload.message);
          setAnswerPending(false);
          recordEvent("error", "ask failed", event.payload.message);
        }
      },
    ).then((unlisten) => unlistens.push(unlisten));

    return () => unlistens.forEach((unlisten) => unlisten());
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
      ".doc-head h1, .doc-lede, .doc-prose, .doc-section h2, .doc-status__row, .doc-specs__entry, .doc-detail, .doc-answer, .doc-quote, .doc-error";

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
  }, [
    runtimeStatus,
    runtimeEvent,
    runtimeError,
    specsIndex,
    specsIndexError,
    specDetail,
    specDetailError,
    answerPrompt,
    answerText,
    answerPending,
    answerError,
    answerGrounding,
    sessionEvents,
  ]);

  async function selectSpecDetail(specId: string) {
    try {
      const detail = await invoke<StaticSpecDetail>("get_static_spec_detail", {
        specId,
      });
      setSpecDetail(detail);
      setSpecDetailError(null);
    } catch (error) {
      setSpecDetailError(formatSpecDetailError(error));
    }
  }

  function clearActiveArtifact() {
    setSpecDetail(null);
    setSpecDetailError(null);
  }

  async function dispatchAsk(question: string) {
    const grounding = specDetail?.specId ?? null;
    const id = (requestCounter.current += 1);
    currentRequest.current = id;
    currentAsk.current = { prompt: question, grounding };
    setAnswerPrompt(question);
    setAnswerText("");
    setAnswerError(null);
    setAnswerGrounding(grounding);
    setAnswerPending(true);
    try {
      await invoke("ask_stream", {
        requestId: id,
        specId: grounding,
        question,
      });
    } catch (error) {
      if (currentRequest.current === id) {
        setAnswerError(formatModelError(error));
        setAnswerPending(false);
      }
    }
  }

  async function submitCommand(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (!shouldSubmitCommandInput(commandValue)) {
      return;
    }

    const rawCommand = commandValue;
    setCommandValue("");

    try {
      const commandRoute = await invoke<CommandRoute>("route_command", {
        input: rawCommand,
      });

      setCommandRoutes((entries) =>
        appendCommandRouteLogEntry(entries, commandRoute),
      );
      recordEvent(
        "command",
        commandRoute.parsed.kind,
        `${commandRoute.disposition} → ${commandRoute.target}`,
      );

      if (isHandledRoute(commandRoute)) {
        const specId = handledSpecDetailId(commandRoute);
        if (specId) {
          await selectSpecDetail(specId);
        }

        const question = handledAskQuestion(commandRoute);
        if (question) {
          void dispatchAsk(question);
        }

        const targetId = routeTargetElementId(commandRoute.target);
        const target = targetId ? document.getElementById(targetId) : null;

        if (!target) {
          setCommandError(
            `Route target unavailable: ${commandRoute.target}.`,
          );
          return;
        }

        target.scrollIntoView({ block: "start" });
      }

      setCommandError(null);
    } catch (error) {
      setCommandError(formatCommandError(error));
    }
  }

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
          <a className="doc-nav__link" href="#specs">
            Specs
          </a>
          <a className="doc-nav__link" href="#spec-detail">
            Spec detail
          </a>
          <a className="doc-nav__link" href="#answer">
            Answer
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

        <section className="doc-section" id="specs">
          <h2>Specs</h2>
          {specsIndex ? (
            <>
              <p className="doc-prose">{specsIndex.summary}</p>
              <p className="doc-prose doc-prose--muted doc-prose--mono">
                {formatSpecsIndexSource(specsIndex)}
              </p>
              <ol className="doc-specs" aria-label="Specs index">
                {specsIndex.specs.map((spec) => (
                  <li
                    className="doc-specs__entry"
                    key={spec.specId}
                    aria-current={
                      specDetail?.specId === spec.specId ? "true" : undefined
                    }
                  >
                    <div className="doc-specs__head">
                      <span>{spec.specId}</span>
                      <em>{spec.status}</em>
                    </div>
                    <h3>{spec.title}</h3>
                    <dl className="doc-specs__meta">
                      <div>
                        <dt>path</dt>
                        <dd>{spec.path}</dd>
                      </div>
                      <div>
                        <dt>branch</dt>
                        <dd>{spec.implementationBranch}</dd>
                      </div>
                      <div>
                        <dt>route</dt>
                        <dd>{spec.route}</dd>
                      </div>
                    </dl>
                    <button
                      type="button"
                      className="doc-specs__select"
                      aria-label={`Select ${spec.specId} ${spec.title}`}
                      aria-pressed={specDetail?.specId === spec.specId}
                      onClick={() => selectSpecDetail(spec.specId)}
                    >
                      View detail
                    </button>
                  </li>
                ))}
              </ol>
            </>
          ) : specsIndexError ? (
            <div className="doc-error" role="status">
              <strong>Specs index unavailable</strong>
              <span>{specsIndexError}</span>
            </div>
          ) : (
            <p className="doc-prose doc-prose--muted" role="status">
              Loading the static specs index…
            </p>
          )}
        </section>

        <section className="doc-section" id="spec-detail">
          <h2>Spec detail</h2>
          {specDetailError ? (
            <div className="doc-error" role="status">
              <strong>Spec detail unavailable</strong>
              <span>{specDetailError}</span>
            </div>
          ) : null}
          {specDetail ? (
            <article className="doc-detail">
              <div className="doc-specs__head">
                <span>{specDetail.specId}</span>
                <em>{specDetail.status}</em>
              </div>
              <h3>{specDetail.title}</h3>
              <p className="doc-prose">{specDetail.summary}</p>
              <dl className="doc-specs__meta">
                <div>
                  <dt>path</dt>
                  <dd>{specDetail.path}</dd>
                </div>
                <div>
                  <dt>branch</dt>
                  <dd>{specDetail.implementationBranch}</dd>
                </div>
                <div>
                  <dt>route</dt>
                  <dd>{specDetail.route}</dd>
                </div>
              </dl>
              <div className="doc-detail__lists">
                <div>
                  <h4>Scope</h4>
                  <ul>
                    {specDetail.scope.map((item) => (
                      <li key={item}>{item}</li>
                    ))}
                  </ul>
                </div>
                <div>
                  <h4>Limitations</h4>
                  <ul>
                    {specDetail.limitations.map((item) => (
                      <li key={item}>{item}</li>
                    ))}
                  </ul>
                </div>
              </div>
            </article>
          ) : !specDetailError ? (
            <p className="doc-prose doc-prose--muted" role="status">
              Select a spec from the index or run /specs/&lt;id&gt; to see its
              static detail.
            </p>
          ) : null}
        </section>

        <section className="doc-section" id="answer">
          <h2>Answer</h2>
          {answerError ? (
            <div className="doc-error" role="status">
              <strong>Model unavailable</strong>
              <span>{answerError}</span>
            </div>
          ) : answerPrompt !== null ? (
            <article className="doc-answer">
              <p className="doc-answer__prompt doc-prose--mono">
                {answerPrompt}
                {answerGrounding ? (
                  <span className="doc-answer__grounding">
                    {answerGrounding}
                  </span>
                ) : null}
              </p>
              {answerText ? (
                <p className="doc-prose">{answerText}</p>
              ) : answerPending ? (
                <p className="doc-prose doc-prose--muted" role="status">
                  Thinking…
                </p>
              ) : null}
            </article>
          ) : (
            <p className="doc-prose doc-prose--muted" role="status">
              Ask with ? followed by a question.
            </p>
          )}
        </section>

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
          <p className="doc-provider doc-prose--mono" role="status">
            {providerStatus?.model ?? "ollama"} ·{" "}
            {formatProviderState(providerStatus)}
          </p>
        </section>

        <section className="doc-section" id="event-stream">
          <h2>Event stream</h2>
          <blockquote className="doc-quote">
            {runtimeEvent?.eventType ?? "No runtime event yet"}
            <cite>{runtimeEvent?.eventId ?? RUNTIME_STATUS_EVENT}</cite>
          </blockquote>
          {sessionEvents.length > 0 ? (
            <ol className="doc-events" aria-label="Session event log">
              {sessionEvents.map((event) => (
                <li
                  className="doc-events__entry"
                  data-kind={event.kind}
                  key={event.id}
                >
                  {formatSessionEvent(event)}
                </li>
              ))}
            </ol>
          ) : (
            <p className="doc-prose doc-prose--muted" role="status">
              Session activity will appear here.
            </p>
          )}
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
        <div className="doc-dock__inner">
          {commandError ? (
            <div className="doc-command-error" role="status">
              <strong>Command unavailable</strong>
              <span>{commandError}</span>
            </div>
          ) : null}

          {commandRoutes.length > 0 ? (
            <ol
              className="doc-command-log"
              aria-label="Command route log"
              aria-live="polite"
            >
              {commandRoutes.map((route, index) => (
                <li
                  className="doc-command-log__entry"
                  data-latest={index === 0 ? "true" : undefined}
                  data-disposition={route.disposition}
                  key={`${route.parsed.raw}-${index}`}
                >
                  <div className="doc-command-log__meta">
                    <span>{route.parsed.kind}</span>
                    {index === 0 ? <em>latest</em> : null}
                  </div>
                  <dl className="doc-command-log__details">
                    <div>
                      <dt>argument</dt>
                      <dd>{formatCommandArgument(route.parsed.argument)}</dd>
                    </div>
                    <div>
                      <dt>requiresApproval</dt>
                      <dd>
                        {String(route.parsed.requiresApproval)} ·{" "}
                        {formatApprovalState(route.parsed)}
                      </dd>
                    </div>
                    <div>
                      <dt>disposition</dt>
                      <dd>{route.disposition}</dd>
                    </div>
                    <div>
                      <dt>target</dt>
                      <dd>{route.target}</dd>
                    </div>
                  </dl>
                  <p>{route.message}</p>
                </li>
              ))}
            </ol>
          ) : null}

          <form
            className="doc-input"
            aria-label="Command dock"
            onSubmit={submitCommand}
          >
            <span className="doc-input__prefix">&gt;</span>
            <input
              aria-label="Command input"
              placeholder="Type /, ?, @, #, !, >, or natural language…"
              value={commandValue}
              onChange={(event) => {
                setCommandValue(event.target.value);
                setCommandError(null);
              }}
              autoComplete="off"
              spellCheck={false}
            />
            <span className="doc-input__caret" aria-hidden="true" />
          </form>
        </div>
      </div>

      <footer className="doc-foot">
        <div className="doc-foot__status" aria-live="polite">
          <span className="doc-foot__label">artifact</span>
          <span
            className={
              specDetail
                ? "doc-foot__artifact"
                : "doc-foot__artifact doc-foot__artifact--none"
            }
          >
            {formatActiveArtifact(specDetail)}
          </span>
          {specDetail ? (
            <button
              type="button"
              className="doc-foot__clear"
              aria-label={`Clear active artifact ${specDetail.specId}`}
              onClick={clearActiveArtifact}
            >
              clear
            </button>
          ) : null}
        </div>
        <div className="doc-foot__keys">
          <span>
            <kbd>⌘F</kbd> find
          </span>
          <span>
            <kbd>e</kbd> edit
          </span>
          <span>
            <kbd>esc</kbd> back
          </span>
        </div>
      </footer>
    </main>
  );
}

export default App;
