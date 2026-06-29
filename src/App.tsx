import { useEffect, useRef, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import {
  appendCommandRouteLogEntry,
  appendSessionEvent,
  formatActiveArtifact,
  formatApprovalState,
  formatClassificationGate,
  formatCommandError,
  formatCommandArgument,
  formatGitStatus,
  formatLabel,
  formatModelError,
  formatPlanningBaseline,
  formatProviderState,
  formatRuntimeError,
  formatSessionEvent,
  formatSpecDetailError,
  formatSpecsIndexError,
  formatSpecsIndexSource,
  handledAskQuestion,
  handledClassificationRequest,
  handledCommand,
  handledSpecDetailId,
  inertiaStep,
  lineSpreadOffset,
  routeTargetElementId,
  isHandledRoute,
  shouldSubmitCommandInput,
  type CommandRoute,
  type GitStatus,
  type PlanningBaseline,
  type ProviderStatus,
  type RequestClassification,
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

type Workspace = {
  root: string;
  name: string;
};

type WorkspaceDoc = {
  kind: string;
  path: string;
  text: string;
};

type WorkspaceSpec = {
  specId: string;
  path: string;
};

type GitCommit = {
  short: string;
  subject: string;
};

type GitDiff = {
  isRepo: boolean;
  empty: boolean;
  lines: Array<{ kind: string; text: string }>;
};

type ApprovalRequest = {
  id: number;
  action: string;
  summary: string;
  command: string;
  autoApprove: boolean;
};

type ApprovalOutcome = {
  id: number;
  approved: boolean;
  message: string;
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
  const [autonomyMode, setAutonomyMode] = useState("supervised");

  async function toggleAutonomy() {
    const next =
      autonomyMode === "supervised" ? "high_autonomy" : "supervised";
    try {
      const result = await invoke<{ mode: string }>("set_autonomy_mode", {
        mode: next,
      });
      setAutonomyMode(result.mode);
    } catch {
      /* keep the current mode on failure */
    }
  }
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
  const [providerKind, setProviderKind] = useState("ollama");
  const [providerBaseUrl, setProviderBaseUrl] = useState("");
  const [providerModel, setProviderModel] = useState("");
  const [providerApiKey, setProviderApiKey] = useState("");
  const [providerConfigError, setProviderConfigError] = useState<string | null>(
    null,
  );
  const [modelRoles, setModelRoles] = useState<
    Array<{ role: string; model: string; overridden: boolean }>
  >([]);
  const [roleInputs, setRoleInputs] = useState<Record<string, string>>({});
  const [extensions, setExtensions] = useState<
    Array<{ id: number; name: string; kind: string; command: string }>
  >([]);
  const [extName, setExtName] = useState("");
  const [extKind, setExtKind] = useState("tool");
  const [extCommand, setExtCommand] = useState("");
  const [extError, setExtError] = useState<string | null>(null);

  async function refreshExtensions() {
    try {
      setExtensions(
        await invoke<
          Array<{ id: number; name: string; kind: string; command: string }>
        >("list_extensions"),
      );
    } catch {
      setExtensions([]);
    }
  }

  async function registerExtension() {
    setExtError(null);
    try {
      await invoke("register_extension", {
        name: extName,
        kind: extKind,
        command: extCommand,
      });
      setExtName("");
      setExtCommand("");
      await refreshExtensions();
    } catch (error) {
      setExtError(
        error instanceof Error ? error.message : "Could not register extension.",
      );
    }
  }

  async function removeExtension(id: number) {
    try {
      await invoke("remove_extension", { id });
      await refreshExtensions();
    } catch (error) {
      setExtError(
        error instanceof Error ? error.message : "Could not remove extension.",
      );
    }
  }

  async function refreshModelRoles() {
    try {
      setModelRoles(
        await invoke<
          Array<{ role: string; model: string; overridden: boolean }>
        >("get_model_roles"),
      );
    } catch {
      setModelRoles([]);
    }
  }

  async function saveModelRole(role: string) {
    try {
      await invoke("set_model_role", { role, model: roleInputs[role] ?? "" });
      setRoleInputs((inputs) => ({ ...inputs, [role]: "" }));
      void refreshModelRoles();
    } catch {
      /* keep current roles on failure */
    }
  }

  async function saveProviderConfig() {
    try {
      await invoke<{ kind: string; baseUrl: string; model: string }>(
        "set_provider_config",
        {
          kind: providerKind,
          baseUrl: providerBaseUrl.trim(),
          model: providerModel.trim(),
          apiKey: providerApiKey.trim() || null,
        },
      );
      setProviderConfigError(null);
      setProviderApiKey("");
      setProviderStatus(await invoke<ProviderStatus>("get_provider_status"));
    } catch (error) {
      setProviderConfigError(
        error instanceof Error ? error.message : "Could not save provider.",
      );
    }
  }
  const [answerPrompt, setAnswerPrompt] = useState<string | null>(null);
  const [answerText, setAnswerText] = useState("");
  const [answerPending, setAnswerPending] = useState(false);
  const [answerError, setAnswerError] = useState<string | null>(null);
  const [answerGrounding, setAnswerGrounding] = useState<string | null>(null);
  const [classification, setClassification] =
    useState<RequestClassification | null>(null);
  const [classificationRequest, setClassificationRequest] = useState("");
  const [specDraft, setSpecDraft] = useState<string | null>(null);
  const [draftPending, setDraftPending] = useState(false);
  const [draftError, setDraftError] = useState<string | null>(null);
  const [saveSpecId, setSaveSpecId] = useState("");

  async function requestSaveSpec() {
    const specId = saveSpecId.trim();
    if (!specId || !specDraft) {
      return;
    }
    try {
      const request = await invoke<ApprovalRequest>("request_save_spec", {
        specId,
        content: specDraft,
      });
      setPendingApproval(request);
      setApprovalNotice(null);
      recordEvent("command", "approval", `requested ${request.command}`);
    } catch (error) {
      setApprovalNotice(
        error instanceof Error ? error.message : "Could not request save.",
      );
    }
  }

  async function draftSpec() {
    if (!classificationRequest) {
      return;
    }
    setDraftPending(true);
    setDraftError(null);
    setSpecDraft(null);
    try {
      const result = await invoke<{ request: string; draft: string }>(
        "draft_spec",
        { request: classificationRequest },
      );
      setSpecDraft(result.draft);
    } catch (error) {
      setDraftError(
        error instanceof Error ? error.message : "Could not draft a spec.",
      );
    } finally {
      setDraftPending(false);
    }
  }
  const requestCounter = useRef(0);
  const currentRequest = useRef(0);
  const currentAsk = useRef<{ prompt: string; grounding: string | null } | null>(
    null,
  );
  const [sessionEvents, setSessionEvents] = useState<SessionEvent[]>([]);
  const eventCounter = useRef(0);
  const [signals, setSignals] = useState<
    Array<{ kind: string; summary: string; count: number }>
  >([]);
  const [workspace, setWorkspace] = useState<Workspace | null>(null);
  const [workspaceError, setWorkspaceError] = useState<string | null>(null);
  const [baseline, setBaseline] = useState<PlanningBaseline | null>(null);
  const [doc, setDoc] = useState<WorkspaceDoc | null>(null);
  const [docError, setDocError] = useState<string | null>(null);
  const [workspaceSpecs, setWorkspaceSpecs] = useState<WorkspaceSpec[]>([]);
  const [gitStatus, setGitStatus] = useState<GitStatus | null>(null);
  const [gitLog, setGitLog] = useState<GitCommit[]>([]);
  const [gitDiff, setGitDiff] = useState<GitDiff | null>(null);
  const [diffReview, setDiffReview] = useState<string | null>(null);
  const [reviewPending, setReviewPending] = useState(false);
  const [reviewError, setReviewError] = useState<string | null>(null);

  async function reviewDiff() {
    setReviewPending(true);
    setReviewError(null);
    setDiffReview(null);
    try {
      const result = await invoke<{ empty: boolean; review: string }>(
        "review_diff",
      );
      setDiffReview(result.empty ? "No changes to review." : result.review);
      recordEvent("command", "review", "reviewed diff");
    } catch (error) {
      setReviewError(
        error instanceof Error ? error.message : "Could not review the diff.",
      );
    } finally {
      setReviewPending(false);
    }
  }
  const [branchName, setBranchName] = useState("");
  const [commitMessage, setCommitMessage] = useState("");
  const [switchTarget, setSwitchTarget] = useState("");
  const [pushRemote, setPushRemote] = useState("");
  const [prTitle, setPrTitle] = useState("");
  const [prBody, setPrBody] = useState("");
  const [amendSpecId, setAmendSpecId] = useState("");
  const [amendId, setAmendId] = useState("");
  const [amendContent, setAmendContent] = useState("");

  async function requestRunCommand(command: string) {
    try {
      const request = await invoke<ApprovalRequest>("request_run_command", {
        command,
      });
      setPendingApproval(request);
      setApprovalNotice(null);
      recordEvent("command", "approval", `requested run ${request.command}`);
    } catch (error) {
      setApprovalNotice(
        error instanceof Error ? error.message : "Could not request command.",
      );
    }
  }

  async function requestSaveAmendment() {
    const specId = amendSpecId.trim();
    const amendmentId = amendId.trim();
    if (!specId || !amendmentId || !amendContent.trim()) {
      return;
    }
    try {
      const request = await invoke<ApprovalRequest>("request_save_amendment", {
        specId,
        amendmentId,
        content: amendContent,
      });
      setPendingApproval(request);
      setApprovalNotice(null);
      recordEvent("command", "approval", `requested ${request.command}`);
    } catch (error) {
      setApprovalNotice(
        error instanceof Error
          ? error.message
          : "Could not request amendment.",
      );
    }
  }
  const [pendingApproval, setPendingApproval] =
    useState<ApprovalRequest | null>(null);
  const [approvalNotice, setApprovalNotice] = useState<string | null>(null);

  async function requestBranch() {
    const name = branchName.trim();
    if (!name) {
      return;
    }
    try {
      const request = await invoke<ApprovalRequest>("request_create_branch", {
        name,
      });
      setPendingApproval(request);
      setApprovalNotice(null);
      recordEvent("command", "approval", `requested ${request.command}`);
    } catch (error) {
      setApprovalNotice(
        error instanceof Error ? error.message : "Could not request branch.",
      );
    }
  }

  async function requestCommit() {
    const message = commitMessage.trim();
    if (!message) {
      return;
    }
    try {
      const request = await invoke<ApprovalRequest>("request_commit", {
        message,
      });
      setPendingApproval(request);
      setApprovalNotice(null);
      recordEvent("command", "approval", `requested ${request.action}`);
    } catch (error) {
      setApprovalNotice(
        error instanceof Error ? error.message : "Could not request commit.",
      );
    }
  }

  async function requestSwitch() {
    const name = switchTarget.trim();
    if (!name) {
      return;
    }
    try {
      const request = await invoke<ApprovalRequest>("request_switch_branch", {
        name,
      });
      setPendingApproval(request);
      setApprovalNotice(null);
      recordEvent("command", "approval", `requested ${request.command}`);
    } catch (error) {
      setApprovalNotice(
        error instanceof Error ? error.message : "Could not request switch.",
      );
    }
  }

  async function requestPush() {
    try {
      const request = await invoke<ApprovalRequest>("request_push", {
        remote: pushRemote.trim(),
      });
      setPendingApproval(request);
      setApprovalNotice(null);
      recordEvent("command", "approval", `requested ${request.command}`);
    } catch (error) {
      setApprovalNotice(
        error instanceof Error ? error.message : "Could not request push.",
      );
    }
  }

  async function requestPr() {
    const title = prTitle.trim();
    if (!title) {
      return;
    }
    try {
      const request = await invoke<ApprovalRequest>("request_create_pr", {
        title,
        body: prBody,
      });
      setPendingApproval(request);
      setApprovalNotice(null);
      recordEvent("command", "approval", `requested ${request.action}`);
    } catch (error) {
      setApprovalNotice(
        error instanceof Error ? error.message : "Could not request PR.",
      );
    }
  }

  useEffect(() => {
    if (pendingApproval?.autoApprove) {
      void resolveApproval(true);
    }
  }, [pendingApproval]);

  async function resolveApproval(approved: boolean) {
    if (!pendingApproval) {
      return;
    }
    const id = pendingApproval.id;
    try {
      const outcome = await invoke<ApprovalOutcome>("resolve_approval", {
        id,
        approved,
      });
      setPendingApproval(null);
      setApprovalNotice(outcome.message);
      recordEvent(
        approved ? "command" : "error",
        "approval",
        outcome.message,
      );
      if (approved) {
        setBranchName("");
        setCommitMessage("");
        setSwitchTarget("");
        setPushRemote("");
        setPrTitle("");
        setPrBody("");
        setSaveSpecId("");
        setAmendSpecId("");
        setAmendId("");
        setAmendContent("");
        void refreshBaseline();
      }
    } catch (error) {
      setPendingApproval(null);
      setApprovalNotice(
        error instanceof Error ? error.message : "Approval failed.",
      );
    }
  }

  async function viewDoc(kind: string) {
    try {
      const opened = await invoke<WorkspaceDoc>("read_workspace_doc", { kind });
      setDoc(opened);
      setDocError(null);
      recordEvent("command", "read", opened.path);
    } catch (error) {
      setDocError(
        error instanceof Error ? error.message : "Could not read document.",
      );
    }
  }

  async function refreshBaseline() {
    try {
      setBaseline(
        await invoke<PlanningBaseline>("inspect_planning_baseline"),
      );
    } catch {
      setBaseline(null);
    }
    try {
      setWorkspaceSpecs(
        await invoke<WorkspaceSpec[]>("list_workspace_specs"),
      );
    } catch {
      setWorkspaceSpecs([]);
    }
    try {
      setGitStatus(await invoke<GitStatus>("git_status"));
    } catch {
      setGitStatus(null);
    }
    try {
      setGitLog(await invoke<GitCommit[]>("git_log"));
    } catch {
      setGitLog([]);
    }
    try {
      setGitDiff(await invoke<GitDiff>("git_diff"));
    } catch {
      setGitDiff(null);
    }
  }

  function recordEvent(
    kind: SessionEventKind,
    label: string,
    detail: string,
  ) {
    const id = (eventCounter.current += 1);
    setSessionEvents((events) =>
      appendSessionEvent(events, { id, kind, label, detail }),
    );
    void invoke("append_event", { kind, label, detail }).catch(() => {});
  }

  async function openWorkspace() {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (typeof selected !== "string") {
        return;
      }
      const opened = await invoke<Workspace>("open_workspace", {
        path: selected,
      });
      setWorkspace(opened);
      setWorkspaceError(null);
      recordEvent("command", "workspace", `opened ${opened.name}`);
      void refreshBaseline();
    } catch (error) {
      setWorkspaceError(
        error instanceof Error ? error.message : "Could not open workspace.",
      );
    }
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

    invoke<{ kind: string; baseUrl: string; model: string }>(
      "get_provider_config",
    )
      .then((config) => {
        if (active) {
          setProviderKind(config.kind);
          setProviderBaseUrl(config.baseUrl);
          setProviderModel(config.model);
        }
      })
      .catch(() => {});

    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    invoke<{ mode: string }>("get_autonomy_mode")
      .then((result) => setAutonomyMode(result.mode))
      .catch(() => {});
    void refreshModelRoles();
    void refreshExtensions();
    invoke<SessionEvent[]>("load_events", { limit: 50 })
      .then((records) => {
        if (records.length) {
          setSessionEvents(records);
          eventCounter.current = records[0]?.id ?? 0;
        }
      })
      .catch(() => {});
  }, []);

  useEffect(() => {
    invoke<Array<{ kind: string; summary: string; count: number }>>(
      "improvement_signals",
    )
      .then(setSignals)
      .catch(() => {});
  }, [sessionEvents]);

  useEffect(() => {
    let active = true;
    invoke<Workspace | null>("get_workspace")
      .then((opened) => {
        if (active && opened) {
          setWorkspace(opened);
          void refreshBaseline();
        }
      })
      .catch(() => {});
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

    const view = scroller;
    let target = view.scrollTop;
    let raf = 0;
    let animating = false;

    const clamp = (value: number) =>
      Math.max(0, Math.min(value, view.scrollHeight - view.clientHeight));

    const tick = () => {
      const next = inertiaStep(view.scrollTop, target);
      view.scrollTop = next;
      if (next === target) {
        animating = false;
        raf = 0;
        return;
      }
      raf = requestAnimationFrame(tick);
    };

    const onWheel = (event: WheelEvent) => {
      if (event.ctrlKey) {
        return;
      }
      const unit =
        event.deltaMode === 1
          ? 16
          : event.deltaMode === 2
            ? view.clientHeight
            : 1;
      const next = clamp(target + event.deltaY * unit);
      if (next === target) {
        return;
      }
      event.preventDefault();
      target = next;
      if (!animating) {
        animating = true;
        raf = requestAnimationFrame(tick);
      }
    };

    const syncTarget = () => {
      if (!animating) {
        target = view.scrollTop;
      }
    };

    view.addEventListener("wheel", onWheel, { passive: false });
    view.addEventListener("scroll", syncTarget, { passive: true });

    return () => {
      if (raf) {
        cancelAnimationFrame(raf);
      }
      view.removeEventListener("wheel", onWheel);
      view.removeEventListener("scroll", syncTarget);
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
      ".doc-head h1, .doc-lede, .doc-prose, .doc-section h2, .doc-status__row, .doc-specs__entry, .doc-detail, .doc-answer, .doc-classification, .doc-quote, .doc-error";

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
    classification,
    specDraft,
    draftPending,
    draftError,
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

        const request = handledClassificationRequest(commandRoute);
        if (request) {
          setClassificationRequest(request);
          setSpecDraft(null);
          setDraftError(null);
          void invoke<RequestClassification>("classify_request", {
            input: request,
          })
            .then((result) => setClassification(result))
            .catch(() => setClassification(null));
        }

        const command = handledCommand(commandRoute);
        if (command) {
          void requestRunCommand(command);
          setCommandError(null);
          return;
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
          <a className="doc-nav__link" href="#classification">
            Classification
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
          <div className="doc-workspace">
            <span
              className={
                workspace
                  ? "doc-workspace__name"
                  : "doc-workspace__name doc-prose--muted"
              }
              title={workspace?.root}
            >
              {workspace ? workspace.name : "No workspace"}
            </span>
            <button
              type="button"
              className="doc-workspace__open"
              onClick={openWorkspace}
            >
              {workspace ? "Change" : "Open"}
            </button>
            {workspaceError ? (
              <span className="doc-workspace__error">{workspaceError}</span>
            ) : null}
          </div>
          {workspace && baseline ? (
            <p
              className="doc-workspace__baseline doc-prose--mono"
              data-complete={baseline.complete}
            >
              {formatPlanningBaseline(baseline)}
            </p>
          ) : null}
          {workspace && gitStatus ? (
            <p
              className="doc-workspace__git doc-prose--mono"
              data-clean={gitStatus.isRepo ? gitStatus.clean : undefined}
            >
              {formatGitStatus(gitStatus)}
            </p>
          ) : null}
          {workspace && gitStatus?.isRepo ? (
            <div className="doc-workspace__branch">
              <input
                aria-label="New branch name"
                placeholder="new branch name"
                value={branchName}
                spellCheck={false}
                autoComplete="off"
                onChange={(event) => setBranchName(event.target.value)}
              />
              <button type="button" onClick={requestBranch}>
                Create branch
              </button>
            </div>
          ) : null}
          {workspace && gitStatus?.isRepo ? (
            <div className="doc-workspace__branch">
              <input
                aria-label="Commit message"
                placeholder="commit message"
                value={commitMessage}
                spellCheck={false}
                autoComplete="off"
                onChange={(event) => setCommitMessage(event.target.value)}
              />
              <button type="button" onClick={requestCommit}>
                Commit
              </button>
            </div>
          ) : null}
          {workspace && gitStatus?.isRepo ? (
            <div className="doc-workspace__branch">
              <input
                aria-label="Switch to branch"
                placeholder="switch to branch"
                value={switchTarget}
                spellCheck={false}
                autoComplete="off"
                onChange={(event) => setSwitchTarget(event.target.value)}
              />
              <button type="button" onClick={requestSwitch}>
                Switch
              </button>
            </div>
          ) : null}
          {workspace && gitStatus?.isRepo ? (
            <div className="doc-workspace__branch">
              <input
                aria-label="Push remote"
                placeholder="push remote (origin)"
                value={pushRemote}
                spellCheck={false}
                autoComplete="off"
                onChange={(event) => setPushRemote(event.target.value)}
              />
              <button type="button" onClick={requestPush}>
                Push
              </button>
            </div>
          ) : null}
          {workspace && gitStatus?.isRepo ? (
            <div className="doc-workspace__pr">
              <input
                aria-label="Pull request title"
                placeholder="pull request title"
                value={prTitle}
                spellCheck={false}
                autoComplete="off"
                onChange={(event) => setPrTitle(event.target.value)}
              />
              <textarea
                aria-label="Pull request body"
                placeholder="pull request body"
                value={prBody}
                rows={2}
                onChange={(event) => setPrBody(event.target.value)}
              />
              <button type="button" onClick={requestPr}>
                Open PR
              </button>
            </div>
          ) : null}
          {workspace ? (
            <div className="doc-workspace__pr">
              <input
                aria-label="Amendment spec id"
                placeholder="spec id (e.g. FS-005)"
                value={amendSpecId}
                spellCheck={false}
                autoComplete="off"
                onChange={(event) => setAmendSpecId(event.target.value)}
              />
              <input
                aria-label="Amendment id"
                placeholder="amendment id (e.g. AMD-001)"
                value={amendId}
                spellCheck={false}
                autoComplete="off"
                onChange={(event) => setAmendId(event.target.value)}
              />
              <textarea
                aria-label="Amendment content"
                placeholder="amendment content"
                value={amendContent}
                rows={2}
                onChange={(event) => setAmendContent(event.target.value)}
              />
              <button type="button" onClick={requestSaveAmendment}>
                Save amendment
              </button>
            </div>
          ) : null}
          {approvalNotice ? (
            <p className="doc-workspace__notice doc-prose--mono">
              {approvalNotice}
            </p>
          ) : null}
          {workspace && baseline && (baseline.prdPresent || baseline.erdPresent) ? (
            <div className="doc-workspace__docs">
              {baseline.prdPresent ? (
                <button type="button" onClick={() => viewDoc("prd")}>
                  PRD
                </button>
              ) : null}
              {baseline.erdPresent ? (
                <button type="button" onClick={() => viewDoc("erd")}>
                  ERD
                </button>
              ) : null}
            </div>
          ) : null}
        </header>

        {doc || docError ? (
          <section className="doc-section" id="reader">
            <h2>{doc ? doc.path : "Document"}</h2>
            {docError ? (
              <div className="doc-error" role="status">
                <strong>Document unavailable</strong>
                <span>{docError}</span>
              </div>
            ) : doc ? (
              <pre className="doc-reader">{doc.text}</pre>
            ) : null}
            <button
              type="button"
              className="doc-reader__close"
              onClick={() => {
                setDoc(null);
                setDocError(null);
              }}
            >
              close
            </button>
          </section>
        ) : null}

        {workspace && workspaceSpecs.length > 0 ? (
          <section className="doc-section" id="workspace-specs">
            <h2>Workspace specs</h2>
            <ol className="doc-events" aria-label="Workspace specs">
              {workspaceSpecs.map((spec) => (
                <li className="doc-events__entry" key={spec.specId}>
                  {spec.specId} · {spec.path}
                </li>
              ))}
            </ol>
          </section>
        ) : null}

        {workspace && gitDiff?.isRepo && !gitDiff.empty ? (
          <section className="doc-section" id="diff">
            <h2>Diff</h2>
            <pre className="doc-diff" aria-label="Working tree diff">
              {gitDiff.lines.map((line, index) => (
                <div
                  className="doc-diff__line"
                  data-kind={line.kind}
                  key={index}
                >
                  {line.text || " "}
                </div>
              ))}
            </pre>
            <button
              type="button"
              className="doc-reader__close"
              onClick={reviewDiff}
              disabled={reviewPending}
            >
              {reviewPending ? "Reviewing…" : "Review diff"}
            </button>
            {reviewError ? (
              <div className="doc-error" role="status">
                <strong>Review unavailable</strong>
                <span>{reviewError}</span>
              </div>
            ) : diffReview ? (
              <p className="doc-prose">{diffReview}</p>
            ) : null}
          </section>
        ) : null}

        {workspace && gitLog.length > 0 ? (
          <section className="doc-section" id="git-log">
            <h2>Recent commits</h2>
            <ol className="doc-events" aria-label="Recent commits">
              {gitLog.map((commit) => (
                <li className="doc-events__entry" key={commit.short}>
                  {commit.short} · {commit.subject}
                </li>
              ))}
            </ol>
          </section>
        ) : null}

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

        <section className="doc-section" id="classification">
          <h2>Classification</h2>
          {classification ? (
            <div className="doc-classification" data-kind={classification.kind}>
              <p className="doc-classification__head doc-prose--mono">
                {classification.kind} · {formatClassificationGate(classification)}
              </p>
              <p className="doc-prose">{classification.rationale}</p>
              {classification.specRequired ? (
                <button
                  type="button"
                  className="doc-reader__close"
                  onClick={draftSpec}
                  disabled={draftPending}
                >
                  {draftPending ? "Drafting…" : "Draft spec"}
                </button>
              ) : null}
              {draftError ? (
                <div className="doc-error" role="status">
                  <strong>Draft unavailable</strong>
                  <span>{draftError}</span>
                </div>
              ) : specDraft ? (
                <>
                  <pre className="doc-reader">{specDraft}</pre>
                  <div className="doc-workspace__branch">
                    <input
                      aria-label="Spec id to save"
                      placeholder="spec id (e.g. FS-099)"
                      value={saveSpecId}
                      spellCheck={false}
                      autoComplete="off"
                      onChange={(event) => setSaveSpecId(event.target.value)}
                    />
                    <button type="button" onClick={requestSaveSpec}>
                      Save spec
                    </button>
                  </div>
                </>
              ) : null}
            </div>
          ) : (
            <p className="doc-prose doc-prose--muted" role="status">
              Type a request to see how Synth would treat it.
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
          <div className="doc-workspace__branch">
            <select
              aria-label="Provider kind"
              value={providerKind}
              onChange={(event) => setProviderKind(event.target.value)}
            >
              <option value="ollama">ollama</option>
              <option value="openai">openai</option>
            </select>
            <input
              aria-label="Provider base URL"
              placeholder="base url"
              value={providerBaseUrl}
              spellCheck={false}
              autoComplete="off"
              onChange={(event) => setProviderBaseUrl(event.target.value)}
            />
            <input
              aria-label="Provider model"
              placeholder="model"
              value={providerModel}
              spellCheck={false}
              autoComplete="off"
              onChange={(event) => setProviderModel(event.target.value)}
            />
            {providerKind === "openai" ? (
              <input
                aria-label="Provider API key"
                type="password"
                placeholder="api key"
                value={providerApiKey}
                spellCheck={false}
                autoComplete="off"
                onChange={(event) => setProviderApiKey(event.target.value)}
              />
            ) : null}
            <button type="button" onClick={saveProviderConfig}>
              Save
            </button>
          </div>
          {providerConfigError ? (
            <p className="doc-workspace__notice doc-prose--mono">
              {providerConfigError}
            </p>
          ) : null}
          {modelRoles.length > 0 ? (
            <dl className="doc-roles">
              {modelRoles.map((role) => (
                <div className="doc-roles__row" key={role.role}>
                  <dt>{formatLabel(role.role)}</dt>
                  <dd>
                    <span
                      className="doc-roles__model"
                      data-overridden={role.overridden}
                    >
                      {role.model}
                    </span>
                    <input
                      aria-label={`Override model for ${role.role}`}
                      placeholder="override model"
                      value={roleInputs[role.role] ?? ""}
                      spellCheck={false}
                      autoComplete="off"
                      onChange={(event) =>
                        setRoleInputs((inputs) => ({
                          ...inputs,
                          [role.role]: event.target.value,
                        }))
                      }
                    />
                    <button type="button" onClick={() => saveModelRole(role.role)}>
                      Save
                    </button>
                  </dd>
                </div>
              ))}
            </dl>
          ) : null}
        </section>

        <section className="doc-section" id="extensions">
          <h2>Extensions</h2>
          <div className="doc-control">
            <input
              aria-label="Extension name"
              placeholder="name"
              value={extName}
              spellCheck={false}
              autoComplete="off"
              onChange={(event) => setExtName(event.target.value)}
            />
            <select
              aria-label="Extension kind"
              value={extKind}
              onChange={(event) => setExtKind(event.target.value)}
            >
              <option value="tool">tool</option>
              <option value="mcp">mcp</option>
              <option value="skill">skill</option>
            </select>
            <input
              aria-label="Extension command"
              placeholder="command"
              value={extCommand}
              spellCheck={false}
              autoComplete="off"
              onChange={(event) => setExtCommand(event.target.value)}
            />
            <button type="button" onClick={registerExtension}>
              Add
            </button>
          </div>
          {extError ? (
            <p className="doc-workspace__notice doc-prose--mono">{extError}</p>
          ) : null}
          {extensions.length > 0 ? (
            <ul className="doc-extensions" aria-label="Registered extensions">
              {extensions.map((extension) => (
                <li className="doc-extensions__row" key={extension.id}>
                  <span className="doc-extensions__name">{extension.name}</span>
                  <span className="doc-extensions__kind">{extension.kind}</span>
                  <code className="doc-extensions__command">
                    {extension.command}
                  </code>
                  <button
                    type="button"
                    onClick={() => void requestRunCommand(extension.command)}
                  >
                    Run
                  </button>
                  <button
                    type="button"
                    onClick={() => void removeExtension(extension.id)}
                  >
                    Remove
                  </button>
                </li>
              ))}
            </ul>
          ) : null}
        </section>

        <section className="doc-section" id="event-stream">
          <h2>Event stream</h2>
          {signals.length > 0 ? (
            <ul className="doc-signals" aria-label="Improvement signals">
              {signals.map((signal) => (
                <li
                  className="doc-signals__item"
                  data-kind={signal.kind}
                  key={signal.kind}
                >
                  {signal.summary}
                </li>
              ))}
            </ul>
          ) : null}
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

      {pendingApproval && !pendingApproval.autoApprove ? (
        <div
          className="doc-approval"
          role="dialog"
          aria-modal="true"
          aria-label="Approval required"
        >
          <div className="doc-approval__panel">
            <p className="doc-approval__title">Approve action</p>
            <p className="doc-approval__summary">{pendingApproval.summary}</p>
            <code className="doc-approval__command">
              {pendingApproval.command}
            </code>
            <div className="doc-approval__actions">
              <button
                type="button"
                className="doc-approval__deny"
                onClick={() => resolveApproval(false)}
              >
                Deny
              </button>
              <button
                type="button"
                className="doc-approval__approve"
                onClick={() => resolveApproval(true)}
              >
                Approve
              </button>
            </div>
          </div>
        </div>
      ) : null}

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
          <button
            type="button"
            className="doc-foot__autonomy"
            data-mode={autonomyMode}
            aria-label={`Autonomy mode ${formatLabel(autonomyMode)}; toggle`}
            onClick={toggleAutonomy}
          >
            {formatLabel(autonomyMode)}
          </button>
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
