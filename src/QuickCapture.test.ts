import { describe, expect, it } from "vitest";
import { inferCapture } from "./QuickCapture";
import type { Subject } from "./types";

const subjects: Subject[] = [
  { id: "physics", name: "Physics", level: "SL", groupNumber: 5, syllabusVersion: "Current", currentGrade: 5, targetGrade: 7, confidence: 0.4, accent: "#ef4444" },
  { id: "business", name: "Business Management", level: "HL", groupNumber: 2, syllabusVersion: "Current", currentGrade: 5, targetGrade: 7, confidence: 0.4, accent: "#8b5cf6" },
];

describe("universal quick capture", () => {
  it("extracts scored assessment evidence and a recurring error", () => {
    const result = inferCapture("I got 18/30 on my Physics test because I ran out of time", subjects);
    expect(result.kind).toBe("assessment");
    expect(result.assessment).toMatchObject({ subjectId: "physics", score: 18, maxScore: 30, whyLostMarks: "I ran out of time" });
    expect(result.assessment.errorCategories).toContain("Time management");
  });

  it("recognizes a subject deadline as an action", () => {
    const result = inferCapture("My Business IA is due next week", subjects);
    expect(result.kind).toBe("task");
    expect(result.task.subjectId).toBe("business");
    expect(result.task.effortMinutes).toBe(90);
  });

  it("recognizes simple practice instructions", () => {
    const result = inferCapture("Redo Physics Paper 1 tomorrow", subjects);
    expect(result.kind).toBe("task");
    expect(result.task.evidenceRequirement).toContain("self-marked");
  });
});
