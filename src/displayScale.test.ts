import { describe, expect, it } from "vitest";
import { adjacentDisplayScale, readDisplayScale } from "./displayScale";

describe("display scaling", () => {
  it("defaults to the more readable comfortable scale", () => {
    expect(readDisplayScale({ getItem: () => null })).toBe(1.15);
  });

  it("restores a valid saved scale and ignores unsupported values", () => {
    expect(readDisplayScale({ getItem: () => "1.3" })).toBe(1.3);
    expect(readDisplayScale({ getItem: () => "4" })).toBe(1.15);
  });

  it("moves between presets without going outside the supported range", () => {
    expect(adjacentDisplayScale(1.15, 1)).toBe(1.3);
    expect(adjacentDisplayScale(1, -1)).toBe(1);
    expect(adjacentDisplayScale(1.5, 1)).toBe(1.5);
  });
});
