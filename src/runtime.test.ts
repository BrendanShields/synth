import { describe, expect, it } from "vitest";
import {
  DEFAULT_SPREAD,
  appendCommandRouteLogEntry,
  appendParsedCommandLogEntry,
  formatApprovalState,
  formatCommandError,
  formatCommandArgument,
  formatLabel,
  formatRuntimeError,
  isHandledRoute,
  lineSpreadOffset,
  routeTargetElementId,
  shouldSubmitCommandInput,
  type CommandRoute,
  type ParsedCommand,
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
    expect(routeTargetElementId("runtime-status")).toBe("runtime-status");
    expect(routeTargetElementId("event-stream")).toBe("event-stream");
    expect(routeTargetElementId("phase")).toBe("phase");
    expect(routeTargetElementId("none")).toBeNull();
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
