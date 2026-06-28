import { describe, expect, it } from "vitest";
import {
  DEFAULT_SPREAD,
  NO_ACTIVE_ARTIFACT_LABEL,
  appendCommandRouteLogEntry,
  appendParsedCommandLogEntry,
  appendSessionEvent,
  formatGitStatus,
  formatPlanningBaseline,
  formatSessionEvent,
  formatActiveArtifact,
  formatApprovalState,
  formatCommandError,
  formatCommandArgument,
  formatLabel,
  formatModelError,
  formatProviderState,
  formatRuntimeError,
  formatSpecDetailError,
  handledAskQuestion,
  formatSpecsIndexError,
  formatSpecsIndexSource,
  handledSpecDetailId,
  isHandledRoute,
  lineSpreadOffset,
  routeTargetElementId,
  shouldSubmitCommandInput,
  type CommandRoute,
  type ParsedCommand,
  type ProviderStatus,
  type SessionEvent,
  type SpecsIndex,
  type StaticSpecDetail,
} from "./runtime";

describe("formatLabel", () => {
  it("replaces underscores with spaces", () => {
    expect(formatLabel("not_opened")).toBe("not opened");
    expect(formatLabel("runtime_status_bootstrap")).toBe(
      "runtime status bootstrap",
    );
  });

  it("leaves text without underscores untouched", () => {
    expect(formatLabel("ready")).toBe("ready");
  });
});

describe("formatRuntimeError", () => {
  it("uses the message of an Error", () => {
    expect(formatRuntimeError(new Error("boom"))).toBe("boom");
  });

  it("passes through a string", () => {
    expect(formatRuntimeError("cli plugin missing")).toBe("cli plugin missing");
  });

  it("falls back for unknown shapes", () => {
    expect(formatRuntimeError({ weird: true })).toBe(
      "Runtime status bridge is unavailable.",
    );
    expect(formatRuntimeError(undefined)).toBe(
      "Runtime status bridge is unavailable.",
    );
  });
});

describe("formatCommandError", () => {
  it("uses specific command parser fallback copy for unknown errors", () => {
    expect(formatCommandError(undefined)).toBe("Command parser is unavailable.");
  });

  it("passes through known errors", () => {
    expect(formatCommandError(new Error("command not found"))).toBe(
      "command not found",
    );
    expect(formatCommandError("denied")).toBe("denied");
  });
});

describe("formatSpecsIndexError", () => {
  it("uses specific specs-index fallback copy for unknown errors", () => {
    expect(formatSpecsIndexError(undefined)).toBe("Specs index is unavailable.");
  });

  it("passes through known errors", () => {
    expect(formatSpecsIndexError(new Error("missing command"))).toBe(
      "missing command",
    );
    expect(formatSpecsIndexError("denied")).toBe("denied");
  });
});

describe("command dock helpers", () => {
  const navigateCommand: ParsedCommand = {
    raw: "/specs",
    kind: "navigate",
    verb: "/",
    argument: "specs",
    requiresApproval: false,
    summary:
      "Navigate intent recognized; navigation routing arrives in a later spec.",
  };

  const shellCommand: ParsedCommand = {
    raw: "! cargo test",
    kind: "shell",
    verb: "!",
    argument: "cargo test",
    requiresApproval: true,
    summary:
      "Shell intent recognized; command execution requires approval and is not yet available.",
  };

  const handledRoute: CommandRoute = {
    parsed: navigateCommand,
    disposition: "handled",
    target: "runtime-status",
    message: "Handled slash navigation route to the runtime-status section.",
  };

  const blockedRoute: CommandRoute = {
    parsed: shellCommand,
    disposition: "blocked",
    target: "none",
    message:
      "Shell route blocked; command execution requires approval and is not yet available.",
  };

  it("prepends parsed commands and caps the transient log", () => {
    const existing = [navigateCommand, navigateCommand, navigateCommand];

    expect(appendParsedCommandLogEntry(existing, shellCommand, 2)).toEqual([
      shellCommand,
      navigateCommand,
    ]);
  });

  it("prepends command routes and caps the transient route log", () => {
    const existing = [handledRoute, handledRoute, handledRoute];

    expect(appendCommandRouteLogEntry(existing, blockedRoute, 2)).toEqual([
      blockedRoute,
      handledRoute,
    ]);
  });

  it("formats missing arguments visibly", () => {
    expect(formatCommandArgument("specs")).toBe("specs");
    expect(formatCommandArgument("")).toBe("∅");
  });

  it("formats approval state from the parsed command payload", () => {
    expect(formatApprovalState(shellCommand)).toBe("approval required");
    expect(formatApprovalState(navigateCommand)).toBe("no approval");
  });

  it("identifies empty command submissions as no-ops", () => {
    expect(shouldSubmitCommandInput("")).toBe(false);
    expect(shouldSubmitCommandInput("  \n\t")).toBe(false);
    expect(shouldSubmitCommandInput(" /specs ")).toBe(true);
  });

  it("maps route targets to existing element ids or no target", () => {
    expect(routeTargetElementId("summary")).toBe("summary");
    expect(routeTargetElementId("specs")).toBe("specs");
    expect(routeTargetElementId("spec-detail")).toBe("spec-detail");
    expect(routeTargetElementId("answer")).toBe("answer");
    expect(routeTargetElementId("runtime-status")).toBe("runtime-status");
    expect(routeTargetElementId("event-stream")).toBe("event-stream");
    expect(routeTargetElementId("phase")).toBe("phase");
    expect(routeTargetElementId("none")).toBeNull();
  });

  it("extracts the canonical id only from handled spec-detail routes", () => {
    const specDetailRoute: CommandRoute = {
      parsed: navigateCommand,
      disposition: "handled",
      target: "spec-detail",
      message: "Handled static spec detail route to FS-001.",
      resource: "FS-001",
    };

    expect(handledSpecDetailId(specDetailRoute)).toBe("FS-001");
    expect(handledSpecDetailId(handledRoute)).toBeNull();
    expect(
      handledSpecDetailId({ ...specDetailRoute, resource: undefined }),
    ).toBeNull();
    expect(
      handledSpecDetailId({ ...specDetailRoute, disposition: "unsupported" }),
    ).toBeNull();
  });

  it("identifies only handled non-none routes as navigable", () => {
    expect(isHandledRoute(handledRoute)).toBe(true);
    expect(isHandledRoute(blockedRoute)).toBe(false);
    expect(
      isHandledRoute({
        ...handledRoute,
        disposition: "unsupported",
        target: "none",
      }),
    ).toBe(false);
  });
});

describe("formatSpecDetailError", () => {
  it("uses specific spec-detail fallback copy for unknown errors", () => {
    expect(formatSpecDetailError(undefined)).toBe("Spec detail is unavailable.");
  });

  it("passes through known errors", () => {
    expect(formatSpecDetailError(new Error("No static spec detail"))).toBe(
      "No static spec detail",
    );
    expect(formatSpecDetailError("unknown spec")).toBe("unknown spec");
  });
});

describe("active artifact", () => {
  const detail: StaticSpecDetail = {
    specId: "FS-002",
    title: "Command dock parsing and intent routing",
    status: "Draft for review",
    path: "docs/specs/FS-002/spec.md",
    implementationBranch: "synth/fs-002-command-dock-parsing",
    route: "/specs/FS-002",
    summary: "Teaches the Rust core to classify command-dock input.",
    scope: ["parse_command classifies input"],
    limitations: ["Parsing only"],
  };

  it("labels an active artifact with its canonical id and title", () => {
    expect(formatActiveArtifact(detail)).toBe(
      "FS-002 · Command dock parsing and intent routing",
    );
  });

  it("uses the neutral label when no artifact is active", () => {
    expect(formatActiveArtifact(null)).toBe(NO_ACTIVE_ARTIFACT_LABEL);
    expect(NO_ACTIVE_ARTIFACT_LABEL).toBe("No active artifact");
  });
});

describe("formatGitStatus", () => {
  it("reports a clean repository with its branch", () => {
    expect(
      formatGitStatus({ isRepo: true, branch: "main", clean: true, changes: [] }),
    ).toBe("main · clean");
  });

  it("pluralizes the change count", () => {
    expect(
      formatGitStatus({
        isRepo: true,
        branch: "x",
        clean: false,
        changes: ["?? a"],
      }),
    ).toBe("x · 1 change");
    expect(
      formatGitStatus({
        isRepo: true,
        branch: "x",
        clean: false,
        changes: ["?? a", " M b"],
      }),
    ).toBe("x · 2 changes");
  });

  it("handles detached HEAD and non-repositories", () => {
    expect(
      formatGitStatus({ isRepo: true, branch: "", clean: true, changes: [] }),
    ).toBe("detached HEAD · clean");
    expect(
      formatGitStatus({ isRepo: false, branch: "", clean: true, changes: [] }),
    ).toBe("not a git repository");
  });
});

describe("formatPlanningBaseline", () => {
  it("reports a complete baseline", () => {
    expect(
      formatPlanningBaseline({
        prdPresent: true,
        erdPresent: true,
        complete: true,
      }),
    ).toBe("planning baseline complete");
  });

  it("names the missing documents", () => {
    expect(
      formatPlanningBaseline({
        prdPresent: false,
        erdPresent: true,
        complete: false,
      }),
    ).toBe("planning baseline incomplete — missing PRD");
    expect(
      formatPlanningBaseline({
        prdPresent: false,
        erdPresent: false,
        complete: false,
      }),
    ).toBe("planning baseline incomplete — missing PRD, ERD");
  });
});

describe("session event log", () => {
  const ev = (id: number, kind: SessionEvent["kind"]): SessionEvent => ({
    id,
    kind,
    label: kind,
    detail: `detail-${id}`,
  });

  it("prepends newest first and enforces the cap", () => {
    let log: SessionEvent[] = [];
    log = appendSessionEvent(log, ev(1, "command"), 2);
    log = appendSessionEvent(log, ev(2, "answer"), 2);
    log = appendSessionEvent(log, ev(3, "error"), 2);

    expect(log.map((e) => e.id)).toEqual([3, 2]);
  });

  it("formats command, answer, and error events", () => {
    expect(formatSessionEvent(ev(1, "command"))).toBe("command · detail-1");
    expect(
      formatSessionEvent({ id: 2, kind: "answer", label: "answer", detail: "" }),
    ).toBe("answer");
  });
});

describe("handledAskQuestion", () => {
  const ask: ParsedCommand = {
    raw: "? what is 2 + 2?",
    kind: "ask",
    verb: "?",
    argument: "what is 2 + 2?",
    requiresApproval: false,
    summary: "Ask intent recognized; artifact questions arrive in a later spec.",
  };

  it("returns the question for a handled answer route", () => {
    const route: CommandRoute = {
      parsed: ask,
      disposition: "handled",
      target: "answer",
      message: "Handled ask route to the model.",
    };
    expect(handledAskQuestion(route)).toBe("what is 2 + 2?");
  });

  it("returns null for non-answer or unsupported routes", () => {
    expect(
      handledAskQuestion({
        parsed: ask,
        disposition: "unsupported",
        target: "none",
        message: "",
      }),
    ).toBeNull();
  });
});

describe("formatModelError", () => {
  it("falls back for unknown shapes", () => {
    expect(formatModelError(undefined)).toBe("The model is unavailable.");
  });

  it("passes through known errors", () => {
    expect(formatModelError("Ollama is not reachable")).toBe(
      "Ollama is not reachable",
    );
  });
});

describe("formatProviderState", () => {
  const base: ProviderStatus = {
    kind: "ollama",
    baseUrl: "http://localhost:11434",
    model: "gemma4:e4b",
    state: "reachable",
    modelPresent: true,
    availableModels: ["gemma4:e4b"],
    detail: "",
  };

  it("is connecting before a status arrives", () => {
    expect(formatProviderState(null)).toBe("connecting");
  });

  it("is ready when reachable with the model present", () => {
    expect(formatProviderState(base)).toBe("ready");
  });

  it("flags a reachable provider missing the model", () => {
    expect(formatProviderState({ ...base, modelPresent: false })).toBe(
      "model missing",
    );
  });

  it("is offline when unreachable", () => {
    expect(
      formatProviderState({ ...base, state: "unreachable", modelPresent: false }),
    ).toBe("offline");
  });
});

describe("specs index helpers", () => {
  it("formats the static specs-index source", () => {
    const index: SpecsIndex = {
      artifactType: "specs-index",
      generatedFrom: "static-rust-catalog",
      specs: [],
      summary: "Static specs index.",
    };

    expect(formatSpecsIndexSource(index)).toBe(
      "specs-index · static-rust-catalog",
    );
  });
});

describe("lineSpreadOffset", () => {
  const vh = 1500;
  const upEdge = vh * DEFAULT_SPREAD.upPivot; // 150
  const downEdge = vh * DEFAULT_SPREAD.downPivot; // 1170

  it("is zero in the calm reading zone", () => {
    expect(lineSpreadOffset(upEdge + 1, vh)).toBe(0);
    expect(lineSpreadOffset(vh * 0.5, vh)).toBe(0);
    expect(lineSpreadOffset(downEdge, vh)).toBe(0);
  });

  it("pulls lines below the down pivot downward (positive)", () => {
    expect(lineSpreadOffset(downEdge + 50, vh)).toBeGreaterThan(0);
  });

  it("pulls lines near the top edge upward (negative)", () => {
    expect(lineSpreadOffset(upEdge - 50, vh)).toBeLessThan(0);
  });

  it("spreads more the lower a line sits past the down pivot", () => {
    const near = lineSpreadOffset(downEdge + 20, vh);
    const far = lineSpreadOffset(downEdge + 120, vh);
    expect(far).toBeGreaterThan(near);
  });

  it("clamps the downward pull so it cannot run away past the band", () => {
    const config = DEFAULT_SPREAD;
    const downSpan = Math.max(1, vh - config.bottomInset - downEdge);
    const max = 1.6 * 1.6 * downSpan * config.downStrength;
    expect(lineSpreadOffset(vh * 5, vh)).toBeCloseTo(max);
  });
});
