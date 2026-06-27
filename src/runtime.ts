export function formatLabel(value: string) {
  return value.replace(/_/g, " ");
}

export function formatRuntimeError(error: unknown) {
  if (error instanceof Error) {
    return error.message;
  }

  if (typeof error === "string") {
    return error;
  }

  return "Runtime status bridge is unavailable.";
}

export function formatCommandError(error: unknown) {
  if (error instanceof Error) {
    return error.message;
  }

  if (typeof error === "string") {
    return error;
  }

  return "Command parser is unavailable.";
}

export function formatSpecsIndexError(error: unknown) {
  if (error instanceof Error) {
    return error.message;
  }

  if (typeof error === "string") {
    return error;
  }

  return "Specs index is unavailable.";
}

export type SpecsIndex = {
  artifactType: "specs-index";
  generatedFrom: "static-rust-catalog";
  specs: SpecIndexEntry[];
  summary: string;
};

export type SpecIndexEntry = {
  specId: string;
  title: string;
  status: string;
  path: string;
  implementationBranch: string;
  route: string;
};

export function formatSpecsIndexSource(index: SpecsIndex) {
  return `${index.artifactType} · ${index.generatedFrom}`;
}

export type ParsedCommandKind =
  | "navigate"
  | "ask"
  | "reference"
  | "tag"
  | "shell"
  | "steer"
  | "natural"
  | "empty";

export type ParsedCommand = {
  raw: string;
  kind: ParsedCommandKind;
  verb: string;
  argument: string;
  requiresApproval: boolean;
  summary: string;
};

export type RouteDisposition = "handled" | "unsupported" | "blocked" | "empty";

export type RouteTarget =
  | "summary"
  | "specs"
  | "runtime-status"
  | "event-stream"
  | "phase"
  | "none";

export type CommandRoute = {
  parsed: ParsedCommand;
  disposition: RouteDisposition;
  target: RouteTarget;
  message: string;
};

export const MAX_COMMAND_LOG_ENTRIES = 20;

export function appendCommandRouteLogEntry(
  entries: CommandRoute[],
  entry: CommandRoute,
  maxEntries = MAX_COMMAND_LOG_ENTRIES,
) {
  return [entry, ...entries].slice(0, maxEntries);
}

export function appendParsedCommandLogEntry(
  entries: ParsedCommand[],
  entry: ParsedCommand,
  maxEntries = MAX_COMMAND_LOG_ENTRIES,
) {
  return [entry, ...entries].slice(0, maxEntries);
}

export function shouldSubmitCommandInput(input: string) {
  return input.trim().length > 0;
}

export function formatCommandArgument(argument: string) {
  return argument.length > 0 ? argument : "∅";
}

export function formatApprovalState(command: ParsedCommand) {
  return command.requiresApproval ? "approval required" : "no approval";
}

export function routeTargetElementId(target: RouteTarget) {
  return target === "none" ? null : target;
}

export function isHandledRoute(route: CommandRoute) {
  return route.disposition === "handled" && route.target !== "none";
}

export type SpreadConfig = {
  upStrength: number;
  downStrength: number;
  upPivot: number;
  downPivot: number;
  bottomInset: number;
};

export const DEFAULT_SPREAD: SpreadConfig = {
  upStrength: 0.16,
  downStrength: 0.08,
  upPivot: 0.1,
  downPivot: 0.78,
  bottomInset: 256,
};

export function lineSpreadOffset(
  center: number,
  viewportHeight: number,
  config: SpreadConfig = DEFAULT_SPREAD,
) {
  const upEdge = viewportHeight * config.upPivot;
  const downEdge = viewportHeight * config.downPivot;
  const downSpan = Math.max(1, viewportHeight - config.bottomInset - downEdge);

  if (center > downEdge) {
    const t = Math.min(1.6, (center - downEdge) / downSpan);
    return t * t * downSpan * config.downStrength;
  }

  if (center < upEdge) {
    const t = Math.min(1.4, (upEdge - center) / upEdge);
    return -t * t * upEdge * config.upStrength;
  }

  return 0;
}
