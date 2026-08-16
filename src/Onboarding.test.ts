import { describe, expect, it } from "vitest";
import { parseSubjectAnswer } from "./Onboarding";

describe("conversational subject intake", () => {
  it("extracts a subject, level and current grade from a natural answer", () => {
    expect(parseSubjectAnswer("Math AA HL, currently 5", 0)).toMatchObject({
      name: "Math AA",
      level: "HL",
      groupNumber: 1,
      currentGrade: 5,
      targetGrade: 7,
    });
  });

  it("accepts a compact answer", () => {
    expect(parseSubjectAnswer("English A SL 6", 4)).toMatchObject({
      name: "English A",
      level: "SL",
      groupNumber: 5,
      currentGrade: 6,
    });
  });

  it("rejects answers missing the level or grade", () => {
    expect(parseSubjectAnswer("Physics grade 5", 1)).toBeNull();
    expect(parseSubjectAnswer("Physics HL", 1)).toBeNull();
  });
});
