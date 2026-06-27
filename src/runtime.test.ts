import { describe, expect, it } from "vitest";
import {
  DEFAULT_SPREAD,
  formatLabel,
  formatRuntimeError,
  lineSpreadOffset,
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
