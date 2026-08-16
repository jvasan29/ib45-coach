import { describe, expect, it } from "vitest";
import { answeredQuestionCount, examQuestionNumbers, formatExamTime, remainingExamSeconds } from "./examLogic";

describe("exam session logic", () => {
  it("never returns a negative timer", () => {
    expect(remainingExamSeconds("2026-01-01T00:00:00Z", new Date("2026-01-01T00:00:01Z").getTime())).toBe(0);
  });

  it("formats long and short timers clearly", () => {
    expect(formatExamTime(3723)).toBe("1:02:03");
    expect(formatExamTime(125)).toBe("2:05");
  });

  it("uses the extracted key length or a safe 40-question fallback", () => {
    expect(examQuestionNumbers({ questionCount: 3 })).toEqual([1, 2, 3]);
    expect(examQuestionNumbers({ questionCount: 0 })).toHaveLength(40);
  });

  it("counts unique answered MCQs", () => {
    expect(answeredQuestionCount({ answers: [
      { id:"1",questionNumber:1,mcqChoice:"A",answerText:"",updatedAt:"" },
      { id:"2",questionNumber:1,mcqChoice:"B",answerText:"",updatedAt:"" },
      { id:"3",questionNumber:2,answerText:"",updatedAt:"" },
    ] })).toBe(1);
  });
});
