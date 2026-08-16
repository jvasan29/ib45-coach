import { invoke } from "@tauri-apps/api/core";
import type { DashboardSnapshot } from "./types";

const native = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const demoSnapshot: DashboardSnapshot = {
  onboarded: true,
  profile: {
    id: "demo",
    name: "IB student",
    examSession: "May 2027",
    timezone: "Asia/Bangkok",
    weeklyCapacityMinutes: 960,
    sleepStart: "23:00",
    sleepEnd: "07:00",
    schoolAiPolicy: "Follow the school policy and cite all generated material.",
    onboardingComplete: true,
  },
  subjects: [
    ["Mathematics: AA", "HL", 5, "#2f6fed"],
    ["Physics", "HL", 6, "#7a5af8"],
    ["Economics", "HL", 5, "#0f9f75"],
    ["English A", "SL", 6, "#d97706"],
    ["French B", "SL", 5, "#e5484d"],
    ["Chemistry", "SL", 6, "#1387a3"],
  ].map(([name, level, currentGrade, accent], index) => ({
    id: `demo-${index}`,
    name: String(name),
    level: level as "HL" | "SL",
    groupNumber: index + 1,
    syllabusVersion: "Current",
    currentGrade: Number(currentGrade),
    targetGrade: 7,
    confidence: 0.62 + index * 0.025,
    accent: String(accent),
  })),
  core: {
    tokGrade: "B",
    eeGrade: "B",
    casComplete: false,
    casExperiences: 4,
    casReflections: 9,
    eeWordCount: 2180,
    eeNextMilestone: "Complete analysis draft",
    tokNextMilestone: "Finalize essay claims",
    corePoints: 2,
  },
  projection: { subjectPoints: 33, corePoints: 2, totalPoints: 35, low: 32, high: 38, targetGap: 10, confidence: 0.68, casRisk: true },
  tasks: [
    { id: "t1", subjectId: "demo-0", title: "Redo calculus integration set", rationale: "Three recurring method errors in the last two papers", status: "open", dueAt: new Date(Date.now() + 3_600_000).toISOString(), effortMinutes: 50, expectedImpact: 0.8, priorityScore: 91, evidenceRequirement: "Upload corrected workings" },
    { id: "t2", subjectId: "demo-2", title: "Plan 15-mark macro response", rationale: "Evaluation is the current grade limiter", status: "open", dueAt: new Date(Date.now() + 86_400_000).toISOString(), effortMinutes: 40, expectedImpact: 0.6, priorityScore: 79, evidenceRequirement: "Save outline and self-mark" },
    { id: "t3", subjectId: "demo-1", title: "Mark Paper 2 mechanics questions", rationale: "Turn completed practice into evidence", status: "open", dueAt: new Date(Date.now() + 172_800_000).toISOString(), effortMinutes: 30, expectedImpact: 0.4, priorityScore: 66, evidenceRequirement: "Record error categories" },
  ],
  overdueCount: 1,
  resourceCount: 24977,
  indexedCount: 18420,
  nextDeadline: new Date(Date.now() + 3_600_000).toISOString(),
};

export async function call<T>(command: string, args: Record<string, unknown> = {}, fallback?: T): Promise<T> {
  if (native) return invoke<T>(command, args);
  if (fallback !== undefined) return structuredClone(fallback);
  throw new Error(`${command} requires the installed desktop application.`);
}

export const api = {
  initialize: () => call("initialize_app", {}, demoSnapshot),
  refresh: () => call("initialize_app", {}, demoSnapshot),
  onboard: (input: unknown) => call<DashboardSnapshot>("complete_onboarding", { input }, demoSnapshot),
  native,
};
