import { describe, expect, it } from "vitest";
import { calculateCorePoints, calculateProjection } from "./scoring";

describe("IB Diploma projection", () => {
  it("uses the official TOK and EE matrix", () => {
    expect(calculateCorePoints("A", "A")).toBe(3);
    expect(calculateCorePoints("B", "C")).toBe(2);
    expect(calculateCorePoints("C", "D")).toBe(0);
    expect(calculateCorePoints("A", "E")).toBe(0);
  });

  it("caps the diploma total at 45", () => {
    expect(calculateProjection([7, 7, 7, 7, 7, 7], "A", "A", 1)).toEqual({
      subjectPoints: 42,
      corePoints: 3,
      totalPoints: 45,
      low: 45,
      high: 45,
      targetGap: 0,
    });
  });

  it("widens the range when evidence confidence is low", () => {
    const highConfidence = calculateProjection([5, 5, 5, 5, 5, 5], "C", "C", .9);
    const lowConfidence = calculateProjection([5, 5, 5, 5, 5, 5], "C", "C", .2);
    expect(lowConfidence.high - lowConfidence.low).toBeGreaterThan(highConfidence.high - highConfidence.low);
  });
});
