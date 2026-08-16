import { useEffect, useMemo, useRef, useState } from "react";
import { ArrowLeft, ArrowRight, Check, ClipboardCheck, ListTodo, ShieldCheck, Sparkles, X } from "lucide-react";
import { call } from "./api";
import type { Subject } from "./types";

type CaptureKind = "assessment" | "task";
type CapturePhase = "capture" | "details" | "review";

type AssessmentDraft = {
  subjectId: string;
  title: string;
  component: string;
  score: number;
  maxScore: number;
  occurredAt: string;
  whyLostMarks: string;
  errorCategories: string[];
};

type TaskDraft = {
  subjectId: string;
  title: string;
  rationale: string;
  dueAt: string;
  effortMinutes: number;
  expectedImpact: number;
  evidenceRequirement: string;
};

export type CaptureInference = {
  kind: CaptureKind;
  confidence: "high" | "medium";
  assessment: AssessmentDraft;
  task: TaskDraft;
};

type QuickCaptureProps = {
  subjects: Subject[];
  onClose: () => void;
  onSaved: (message: string) => Promise<void>;
};

const errorOptions = ["Knowledge gap", "Interpretation", "Method", "Evidence", "Structure", "Terminology", "Time management", "Careless execution"];

export function QuickCapture({ subjects, onClose, onSaved }: QuickCaptureProps) {
  const [phase, setPhase] = useState<CapturePhase>("capture");
  const [capture, setCapture] = useState("");
  const [kind, setKind] = useState<CaptureKind>("assessment");
  const [confidence, setConfidence] = useState<"high" | "medium">("medium");
  const [assessment, setAssessment] = useState<AssessmentDraft>(() => emptyAssessment(subjects));
  const [task, setTask] = useState<TaskDraft>(() => emptyTask(subjects));
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  const firstInput = useRef<HTMLTextAreaElement>(null);
  const dialogRef = useRef<HTMLElement>(null);
  const title = kind === "assessment" ? assessment.title : task.title;
  const selectedSubject = useMemo(() => subjects.find((subject) => subject.id === (kind === "assessment" ? assessment.subjectId : task.subjectId)), [assessment.subjectId, kind, subjects, task.subjectId]);

  useEffect(() => {
    const previousFocus = document.activeElement as HTMLElement | null;
    firstInput.current?.focus();
    const handleKeys = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
      if (event.key !== "Tab" || !dialogRef.current) return;
      const focusable = [...dialogRef.current.querySelectorAll<HTMLElement>("button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled])")];
      if (!focusable.length) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus(); }
      else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
    };
    window.addEventListener("keydown", handleKeys);
    return () => { window.removeEventListener("keydown", handleKeys); previousFocus?.focus(); };
  }, [onClose]);

  function analyze() {
    if (capture.trim().length < 8) {
      setError("Tell me a little more—for example, include the subject, score, result or deadline.");
      return;
    }
    const inference = inferCapture(capture, subjects);
    setKind(inference.kind);
    setConfidence(inference.confidence);
    setAssessment(inference.assessment);
    setTask(inference.task);
    setError("");
    setPhase("details");
  }

  function continueToReview() {
    setError("");
    if (kind === "assessment") {
      if (!assessment.subjectId) return setError("Which subject was this assessment for?");
      if (!assessment.title.trim()) return setError("Give this assessment a short title.");
      if (assessment.maxScore <= 0 || assessment.score < 0 || assessment.score > assessment.maxScore) return setError("Check the score and maximum mark before continuing.");
    } else if (!task.title.trim()) {
      return setError("Turn this into one clear action before continuing.");
    }
    setPhase("review");
  }

  async function save() {
    setSaving(true);
    setError("");
    try {
      if (kind === "assessment") {
        await call("add_assessment", { input: {
          ...assessment,
          assessmentType: "Test",
          weight: 1,
          ibGrade: null,
          feedback: capture,
          occurredAt: new Date(`${assessment.occurredAt}T12:00:00`).toISOString(),
          attachmentPath: null,
        } });
        await onSaved(`Assessment saved for ${selectedSubject?.name ?? "your subject"}. The projection has been recalibrated.`);
      } else {
        await call("create_task", { input: {
          ...task,
          subjectId: task.subjectId || null,
          dueAt: new Date(task.dueAt).toISOString(),
        } });
        await onSaved("Action added and prioritized against your existing plan.");
      }
      onClose();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setSaving(false);
    }
  }

  return <div className="capture-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}>
    <section ref={dialogRef} className="capture-dialog" role="dialog" aria-modal="true" aria-labelledby="capture-title">
      <header className="capture-header">
        <div><span className="capture-orb"><Sparkles size={18}/></span><div><p className="eyebrow">Universal inbox</p><h2 id="capture-title">Tell me what happened</h2></div></div>
        <button className="icon-button" type="button" onClick={onClose} aria-label="Close quick capture"><X size={18}/></button>
      </header>

      <div className="capture-progress" aria-label={`Quick capture step ${phase === "capture" ? 1 : phase === "details" ? 2 : 3} of 3`}>
        {["Describe", "Clarify", "Review"].map((label, index) => { const active = phase === "capture" ? 0 : phase === "details" ? 1 : 2; return <span className={index <= active ? "is-active" : ""} key={label}><i>{index < active ? <Check size={11}/> : index + 1}</i>{label}</span>; })}
      </div>

      {phase === "capture" && <div className="capture-stage">
        <div className="capture-coach-line"><span>45</span><p>Share a result, deadline, teacher comment, mistake or piece of work in your own words.</p></div>
        <label className="capture-message"><span className="sr-only">What happened?</span><textarea ref={firstInput} rows={6} value={capture} onChange={(event) => setCapture(event.target.value)} placeholder="I got 18/30 on my Physics test and lost marks because I ran out of time…" onKeyDown={(event) => { if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) analyze(); }}/></label>
        <div className="capture-examples"><span>Try:</span>{["Business IA due next week", "French oral went well but my vocabulary was weak", "Redo Math Paper 1 tomorrow"].map((example) => <button type="button" onClick={() => setCapture(example)} key={example}>{example}</button>)}</div>
      </div>}

      {phase === "details" && <div className="capture-stage">
        <div className="capture-classification"><div><Sparkles size={17}/><span>I think this is <strong>{kind === "assessment" ? "assessment evidence" : "an action or deadline"}</strong>.</span><small>{confidence} confidence</small></div><div className="capture-kind-switch"><button className={kind === "assessment" ? "is-selected" : ""} onClick={() => setKind("assessment")} type="button"><ClipboardCheck size={15}/> Assessment</button><button className={kind === "task" ? "is-selected" : ""} onClick={() => setKind("task")} type="button"><ListTodo size={15}/> Action</button></div></div>
        {kind === "assessment" ? (
          <AssessmentDetails subjects={subjects} value={assessment} onChange={setAssessment}/>
        ) : (
          <TaskDetails subjects={subjects} value={task} onChange={setTask}/>
        )}
      </div>}

      {phase === "review" && <div className="capture-stage capture-review">
        <div className="capture-coach-line"><span>45</span><p>Here is exactly what I will add. Nothing else will change.</p></div>
        <article className="capture-review-card">
          <div><span className="capture-review-icon">{kind === "assessment" ? <ClipboardCheck size={19}/> : <ListTodo size={19}/>}</span><div><p className="eyebrow">{kind === "assessment" ? "New evidence" : "New action"}</p><h3>{title}</h3></div></div>
          <dl>{kind === "assessment" ? <><div><dt>Subject</dt><dd>{selectedSubject?.name}</dd></div><div><dt>Score</dt><dd>{assessment.score}/{assessment.maxScore} · {Math.round(assessment.score / assessment.maxScore * 100)}%</dd></div><div><dt>Component</dt><dd>{assessment.component}</dd></div><div><dt>Diagnosis</dt><dd>{assessment.whyLostMarks || "No diagnosis yet"}</dd></div><div><dt>Error tags</dt><dd>{assessment.errorCategories.join(", ") || "None"}</dd></div></> : <><div><dt>Subject</dt><dd>{selectedSubject?.name ?? "Core / general"}</dd></div><div><dt>Due</dt><dd>{new Date(task.dueAt).toLocaleString()}</dd></div><div><dt>Effort</dt><dd>{task.effortMinutes} minutes</dd></div><div><dt>Why</dt><dd>{task.rationale || "Captured from your note"}</dd></div><div><dt>Evidence</dt><dd>{task.evidenceRequirement || "Completion check-in"}</dd></div></>}</dl>
        </article>
        <div className="capture-safety"><ShieldCheck size={16}/><span>Policy code performs the save and records the change locally.</span></div>
      </div>}

      <footer className="capture-footer">
        <button className="button button--ghost" type="button" disabled={phase === "capture" || saving} onClick={() => setPhase(phase === "review" ? "details" : "capture")}><ArrowLeft size={16}/> Back</button>
        <span className="form-error" role="alert">{error}</span>
        {phase === "capture" && <button className="button button--primary" type="button" onClick={analyze}>Understand this <ArrowRight size={16}/></button>}
        {phase === "details" && <button className="button button--primary" type="button" onClick={continueToReview}>Review change <ArrowRight size={16}/></button>}
        {phase === "review" && <button className="button button--primary" type="button" disabled={saving} onClick={save}>{saving ? "Saving…" : "Approve and save"}<Check size={16}/></button>}
      </footer>
    </section>
  </div>;
}

function AssessmentDetails({ subjects, value, onChange }: { subjects: Subject[]; value: AssessmentDraft; onChange: (value: AssessmentDraft) => void }) {
  return <div className="capture-fields">
    <label><span>Which subject?</span><select value={value.subjectId} onChange={(event) => onChange({ ...value, subjectId: event.target.value })}><option value="">Choose subject</option>{subjects.map((subject) => <option value={subject.id} key={subject.id}>{subject.name}</option>)}</select></label>
    <label className="capture-span-two"><span>What should this result be called?</span><input value={value.title} onChange={(event) => onChange({ ...value, title: event.target.value })}/></label>
    <label><span>Score</span><input type="number" min="0" value={value.score} onChange={(event) => onChange({ ...value, score: Number(event.target.value) })}/></label>
    <label><span>Out of</span><input type="number" min="1" value={value.maxScore} onChange={(event) => onChange({ ...value, maxScore: Number(event.target.value) })}/></label>
    <label><span>Date</span><input type="date" value={value.occurredAt} onChange={(event) => onChange({ ...value, occurredAt: event.target.value })}/></label>
    <label><span>Component</span><input value={value.component} onChange={(event) => onChange({ ...value, component: event.target.value })}/></label>
    <label className="capture-span-two"><span>Why were marks lost?</span><input value={value.whyLostMarks} onChange={(event) => onChange({ ...value, whyLostMarks: event.target.value })}/></label>
    <fieldset className="capture-tags capture-span-two"><legend>What kind of error was it?</legend>{errorOptions.map((option) => <button type="button" className={value.errorCategories.includes(option) ? "is-selected" : ""} onClick={() => onChange({ ...value, errorCategories: value.errorCategories.includes(option) ? value.errorCategories.filter((item) => item !== option) : [...value.errorCategories, option] })} key={option}>{option}</button>)}</fieldset>
  </div>;
}

function TaskDetails({ subjects, value, onChange }: { subjects: Subject[]; value: TaskDraft; onChange: (value: TaskDraft) => void }) {
  return <div className="capture-fields">
    <label><span>Subject</span><select value={value.subjectId} onChange={(event) => onChange({ ...value, subjectId: event.target.value })}><option value="">Core / general</option>{subjects.map((subject) => <option value={subject.id} key={subject.id}>{subject.name}</option>)}</select></label>
    <label className="capture-span-two"><span>What is the next clear action?</span><input value={value.title} onChange={(event) => onChange({ ...value, title: event.target.value })}/></label>
    <label><span>Due</span><input type="datetime-local" value={value.dueAt} onChange={(event) => onChange({ ...value, dueAt: event.target.value })}/></label>
    <label><span>Focused minutes</span><input type="number" min="15" step="5" value={value.effortMinutes} onChange={(event) => onChange({ ...value, effortMinutes: Number(event.target.value) })}/></label>
    <label className="capture-span-two"><span>Why does this matter?</span><input value={value.rationale} onChange={(event) => onChange({ ...value, rationale: event.target.value })}/></label>
    <label className="capture-span-two"><span>What will prove it is done?</span><input value={value.evidenceRequirement} onChange={(event) => onChange({ ...value, evidenceRequirement: event.target.value })}/></label>
  </div>;
}

export function inferCapture(text: string, subjects: Subject[]): CaptureInference {
  const lower = text.toLowerCase();
  const scoreMatch = text.match(/(\d+(?:\.\d+)?)\s*(?:\/|out\s+of)\s*(\d+(?:\.\d+)?)/i);
  const assessmentSignals = /\b(got|scored|test|quiz|mock|paper|exam|mark|feedback|oral)\b/i.test(text);
  const taskSignals = /\b(due|deadline|finish|complete|redo|revise|practice|submit|write|prepare|need to|must)\b/i.test(text);
  const kind: CaptureKind = scoreMatch || (assessmentSignals && !taskSignals) ? "assessment" : "task";
  const subject = findSubject(lower, subjects);
  const categories = errorOptions.filter((category) => {
    const signals: Record<string, RegExp> = {
      "Knowledge gap": /didn['’]?t know|knowledge|forgot|content gap/i,
      "Interpretation": /misread|interpret|command term|question/i,
      "Method": /method|working|calculation|approach/i,
      "Evidence": /evidence|example|quote|data/i,
      "Structure": /structure|organis|paragraph|argument/i,
      "Terminology": /terminology|vocabulary|wording/i,
      "Time management": /ran out of time|time management|rushed|too slow/i,
      "Careless execution": /careless|silly mistake|sign error|copied wrong/i,
    };
    return signals[category].test(text);
  });
  const now = new Date();
  const due = inferDueDate(lower, now);
  const cleanTitle = text.trim().replace(/[.!?]+$/, "");
  const assessmentTitle = subject ? `${subject.name} assessment` : cleanTitle.slice(0, 72);
  const score = scoreMatch ? Number(scoreMatch[1]) : 0;
  const maxScore = scoreMatch ? Number(scoreMatch[2]) : 100;
  return {
    kind,
    confidence: (scoreMatch || taskSignals || assessmentSignals) ? "high" : "medium",
    assessment: {
      subjectId: subject?.id ?? "",
      title: assessmentTitle,
      component: /paper\s*2/i.test(text) ? "Paper 2" : /paper\s*1/i.test(text) ? "Paper 1" : /oral/i.test(text) ? "Oral" : "Test",
      score,
      maxScore,
      occurredAt: now.toISOString().slice(0, 10),
      whyLostMarks: extractDiagnosis(text),
      errorCategories: categories,
    },
    task: {
      subjectId: subject?.id ?? "",
      title: cleanTitle.slice(0, 110),
      rationale: text.trim(),
      dueAt: toLocalDateTime(due),
      effortMinutes: /ia|essay|extended/i.test(text) ? 90 : 45,
      expectedImpact: /exam|mock|ia|deadline/i.test(text) ? 0.8 : 0.5,
      evidenceRequirement: /redo|practice|paper/i.test(text) ? "Completed and self-marked work" : "Completion check-in",
    },
  };
}

function emptyAssessment(subjects: Subject[]): AssessmentDraft {
  return { subjectId: subjects[0]?.id ?? "", title: "", component: "Test", score: 0, maxScore: 100, occurredAt: new Date().toISOString().slice(0, 10), whyLostMarks: "", errorCategories: [] };
}

function emptyTask(subjects: Subject[]): TaskDraft {
  return { subjectId: subjects[0]?.id ?? "", title: "", rationale: "", dueAt: toLocalDateTime(new Date(Date.now() + 86_400_000)), effortMinutes: 45, expectedImpact: 0.5, evidenceRequirement: "Completion check-in" };
}

function findSubject(lower: string, subjects: Subject[]) {
  return subjects.find((subject) => {
    const name = subject.name.toLowerCase();
    const tokens = name.split(/[^a-z0-9]+/).filter((token) => token.length >= 3);
    return lower.includes(name) || tokens.some((token) => lower.includes(token));
  });
}

function inferDueDate(lower: string, now: Date) {
  const result = new Date(now);
  result.setHours(18, 0, 0, 0);
  if (lower.includes("today")) return result;
  if (lower.includes("tomorrow")) { result.setDate(result.getDate() + 1); return result; }
  if (lower.includes("next week")) { result.setDate(result.getDate() + 7); return result; }
  const dayNames = ["sunday", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday"];
  const day = dayNames.findIndex((name) => lower.includes(name));
  if (day >= 0) { const distance = (day - result.getDay() + 7) % 7 || 7; result.setDate(result.getDate() + distance); return result; }
  result.setDate(result.getDate() + 1);
  return result;
}

function extractDiagnosis(text: string) {
  const match = text.match(/(?:because|since|but)\s+(.+)$/i);
  return match?.[1]?.replace(/[.!?]+$/, "").trim() ?? "";
}

function toLocalDateTime(value: Date) {
  const local = new Date(value.getTime() - value.getTimezoneOffset() * 60_000);
  return local.toISOString().slice(0, 16);
}
