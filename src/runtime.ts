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
