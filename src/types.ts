export type Subject = {
  id: string;
  name: string;
  level: "HL" | "SL";
  groupNumber: number;
  syllabusVersion: string;
  currentGrade: number;
  targetGrade: number;
  confidence: number;
  accent: string;
};

export type StudentProfile = {
  id: string;
  name: string;
  examSession: string;
  timezone: string;
  weeklyCapacityMinutes: number;
  sleepStart: string;
  sleepEnd: string;
  schoolAiPolicy: string;
  onboardingComplete: boolean;
};

export type CoreProgress = {
  tokGrade: string;
  eeGrade: string;
  casComplete: boolean;
  casExperiences: number;
  casReflections: number;
  eeWordCount: number;
  eeNextMilestone: string;
  tokNextMilestone: string;
  corePoints: number;
};

export type Task = {
  id: string;
  subjectId?: string;
  title: string;
  rationale: string;
  status: string;
  dueAt: string;
  effortMinutes: number;
  expectedImpact: number;
  priorityScore: number;
  evidenceRequirement: string;
  completedAt?: string;
};

export type Projection = {
  subjectPoints: number;
  corePoints: number;
  totalPoints: number;
  low: number;
  high: number;
  targetGap: number;
  confidence: number;
  casRisk: boolean;
};

export type DashboardSnapshot = {
  onboarded: boolean;
  profile?: StudentProfile;
  subjects: Subject[];
  core: CoreProgress;
  projection: Projection;
  tasks: Task[];
  overdueCount: number;
  resourceCount: number;
  indexedCount: number;
  nextDeadline?: string;
};

export type AssessmentRecord = {
  id: string;
  subjectId: string;
  title: string;
  assessmentType: string;
  component: string;
  percentage: number;
  ibGrade?: number;
  occurredAt: string;
  feedback: string;
  whyLostMarks: string;
  errorCategories: string[];
};

export type IndexStatus = {
  running: boolean;
  paused: boolean;
  scanned: number;
  indexed: number;
  skipped: number;
  failed: number;
  currentFile: string;
  startedAt?: string;
};

export type ResourceResult = {
  id: string;
  title: string;
  path: string;
  fileType: string;
  sizeBytes: number;
  subjectHint?: string;
  yearHint?: number;
  extractionState: string;
  snippet: string;
  score: number;
};

export type AiAnalysis = {
  id: string;
  provider: string;
  model: string;
  mode: string;
  summary: string;
  claims: string[];
  uncertainty: string;
  evidence: string[];
  recommendedActions: string[];
  academicIntegrityWarning?: string;
  createdAt: string;
};

export type SecretStatus = {
  openaiConfigured: boolean;
  googleConfigured: boolean;
  googleConnected: boolean;
  ollamaAvailable: boolean;
};

export type CalendarBinding = {
  calendarId: string;
  name: string;
  selected: boolean;
  autoEdit: boolean;
  isCoachCalendar: boolean;
  eventCount: number;
};

export type CalendarStatus = {
  connected: boolean;
  accountEmail?: string;
  lastSyncAt?: string;
  bindings: CalendarBinding[];
};

export type ExamPaperCandidate = {
  id: string;
  title: string;
  path: string;
  subjectHint?: string;
  yearHint?: number;
  detectedMode: "mcq" | "theory";
  suggestedMarkSchemeId?: string;
  suggestedMarkSchemeTitle?: string;
};

export type ExamMarkSchemeCandidate = {
  id: string;
  title: string;
  path: string;
  yearHint?: number;
};

export type ExamLibrary = {
  papers: ExamPaperCandidate[];
  markSchemes: ExamMarkSchemeCandidate[];
};

export type ExamAnswer = {
  id: string;
  questionNumber?: number;
  pageNumber?: number;
  answerText: string;
  mcqChoice?: string;
  x?: number;
  y?: number;
  width?: number;
  height?: number;
  updatedAt: string;
};

export type ExamAttempt = {
  id: string;
  subjectId: string;
  subjectName: string;
  paperDocumentId: string;
  paperTitle: string;
  markSchemeDocumentId?: string;
  markSchemeTitle?: string;
  mode: "mcq" | "theory";
  durationMinutes: number;
  status: "active" | "awaiting_manual" | "graded";
  startedAt: string;
  endsAt: string;
  submittedAt?: string;
  score?: number;
  maxScore?: number;
  percentage?: number;
  manualFeedback: string;
  questionCount: number;
  answers: ExamAnswer[];
};

export type ExamAttemptSummary = Pick<ExamAttempt, "id" | "subjectId" | "subjectName" | "paperTitle" | "mode" | "status" | "startedAt" | "endsAt" | "submittedAt" | "score" | "maxScore" | "percentage">;

export type ExamPdfPayload = {
  documentId: string;
  title: string;
  dataBase64: string;
};

export type ViewId = "dashboard" | "subjects" | "plan" | "resources" | "exam" | "core" | "coach" | "calendar" | "settings";
