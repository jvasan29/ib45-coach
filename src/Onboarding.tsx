import { useEffect, useMemo, useRef, useState } from "react";
import { ArrowLeft, ArrowRight, Check, Clock3, Pencil, ShieldCheck, Sparkles } from "lucide-react";
import type { DashboardSnapshot } from "./types";

type SubjectDraft = {
  name: string;
  level: "HL" | "SL";
  groupNumber: number;
  syllabusVersion: string;
  currentGrade: number;
  targetGrade: number;
};

type ProfileDraft = {
  name: string;
  examSession: string;
  timezone: string;
  weeklyCapacityMinutes: number;
  sleepStart: string;
  sleepEnd: string;
  schoolAiPolicy: string;
};

type OnboardingProps = {
  busy: boolean;
  onComplete: (input: Record<string, unknown>) => Promise<void>;
  initial?: DashboardSnapshot;
  onCancel?: () => void;
};

type QuestionId =
  | "name" | "exam" | "capacity" | "sleepStart" | "sleepEnd"
  | "subject0" | "subject1" | "subject2" | "subject3" | "subject4" | "subject5"
  | "tok" | "ee" | "cas" | "policy";

type Question = {
  id: QuestionId;
  stage: string;
  title: string;
  help: string;
  placeholder?: string;
};

const questions: Question[] = [
  { id: "name", stage: "About you", title: "First, what should I call you?", help: "This only appears inside your private local coach.", placeholder: "Your name" },
  { id: "exam", stage: "About you", title: "Which IB exam session are you working toward?", help: "This anchors countdowns, revision cycles and deadline pressure." },
  { id: "capacity", stage: "Your week", title: "How many focused study hours can you realistically do each week?", help: "Be honest rather than ambitious. The plan will protect you from impossible workloads.", placeholder: "For example, 12" },
  { id: "sleepStart", stage: "Your week", title: "What time should studying stop for sleep?", help: "This becomes a hard scheduling boundary." },
  { id: "sleepEnd", stage: "Your week", title: "And when does your protected sleep normally end?", help: "The coach will never schedule work inside this window." },
  ...Array.from({ length: 6 }, (_, index) => ({
    id: `subject${index}` as QuestionId,
    stage: "Your subjects",
    title: `Tell me about subject ${index + 1} of 6.`,
    help: "Write the subject, level and your current IB grade in one line. Your 45 target will automatically set the target grade to 7.",
    placeholder: index === 0 ? "For example: Math AA HL, currently 5" : "For example: English A SL, currently 6",
  })),
  { id: "tok", stage: "The core", title: "What is your current working grade for TOK?", help: "An estimate is fine; later evidence will recalibrate it." },
  { id: "ee", stage: "The core", title: "What is your current working grade for the Extended Essay?", help: "Choose the closest estimate you have today." },
  { id: "cas", stage: "The core", title: "Are your CAS requirements currently complete?", help: "CAS does not add points, but incomplete CAS puts the diploma at risk." },
  { id: "policy", stage: "Academic integrity", title: "What does your school allow you to use AI for?", help: "Paste or summarize the rule. You can skip this and add it later.", placeholder: "For example: brainstorming is allowed, but assessed writing must be my own…" },
];

const emptySubjects: SubjectDraft[] = Array.from({ length: 6 }, (_, index) => ({
  name: "",
  level: index < 3 ? "HL" : "SL",
  groupNumber: index + 1,
  syllabusVersion: "Current",
  currentGrade: 4,
  targetGrade: 7,
}));

const timeOptions = {
  sleepStart: ["21:30", "22:00", "22:30", "23:00", "23:30", "00:00"],
  sleepEnd: ["05:30", "06:00", "06:30", "07:00", "07:30", "08:00"],
};

export function Onboarding({ busy, onComplete, initial, onCancel }: OnboardingProps) {
  const [questionIndex, setQuestionIndex] = useState(0);
  const [reviewing, setReviewing] = useState(false);
  const [error, setError] = useState("");
  const inputRef = useRef<HTMLInputElement | HTMLTextAreaElement>(null);
  const [profile, setProfile] = useState<ProfileDraft>(() => ({
    name: initial?.profile?.name ?? "",
    examSession: initial?.profile?.examSession ?? "May 2027",
    timezone: initial?.profile?.timezone ?? Intl.DateTimeFormat().resolvedOptions().timeZone ?? "Asia/Bangkok",
    weeklyCapacityMinutes: initial?.profile?.weeklyCapacityMinutes ?? 720,
    sleepStart: initial?.profile?.sleepStart ?? "23:00",
    sleepEnd: initial?.profile?.sleepEnd ?? "07:00",
    schoolAiPolicy: initial?.profile?.schoolAiPolicy ?? "",
  }));
  const [capacityHours, setCapacityHours] = useState(() => String((initial?.profile?.weeklyCapacityMinutes ?? 720) / 60));
  const [subjects, setSubjects] = useState<SubjectDraft[]>(() => initial?.subjects.length === 6 ? initial.subjects.map((subject, index) => ({ name: subject.name, level: subject.level, groupNumber: index + 1, syllabusVersion: subject.syllabusVersion, currentGrade: subject.currentGrade, targetGrade: 7 })) : emptySubjects);
  const [subjectAnswers, setSubjectAnswers] = useState(() => initial?.subjects.length === 6 ? initial.subjects.map((subject) => `${subject.name} ${subject.level}, currently ${subject.currentGrade}`) : ["", "", "", "", "", ""]);
  const [core, setCore] = useState(() => ({ tokGrade: initial?.core.tokGrade ?? "C", eeGrade: initial?.core.eeGrade ?? "C", casComplete: initial?.core.casComplete ?? false }));
  const current = questions[questionIndex];
  const progress = reviewing ? 100 : Math.round(((questionIndex + 1) / questions.length) * 100);
  const hlCount = useMemo(() => subjects.filter((subject) => subject.level === "HL").length, [subjects]);

  useEffect(() => {
    if (!reviewing) window.requestAnimationFrame(() => inputRef.current?.focus());
  }, [questionIndex, reviewing]);

  function goForward() {
    setError("");
    if (questionIndex === questions.length - 1) setReviewing(true);
    else setQuestionIndex((value) => value + 1);
  }

  function goBack() {
    setError("");
    if (reviewing) {
      setReviewing(false);
      setQuestionIndex(questions.length - 1);
    } else if (questionIndex > 0) {
      setQuestionIndex((value) => value - 1);
    } else {
      onCancel?.();
    }
  }

  function choose(update: () => void) {
    update();
    window.setTimeout(goForward, 120);
  }

  function submitText() {
    setError("");
    if (current.id === "name") {
      if (!profile.name.trim()) return setError("Tell me what you would like the coach to call you.");
    } else if (current.id === "capacity") {
      const hours = Number(capacityHours);
      if (!Number.isFinite(hours) || hours < 3 || hours > 40) return setError("Enter a realistic number from 3 to 40 hours per week.");
      setProfile((value) => ({ ...value, weeklyCapacityMinutes: Math.round(hours * 60) }));
    } else if (current.id.startsWith("subject")) {
      const index = Number(current.id.replace("subject", ""));
      const parsed = parseSubjectAnswer(subjectAnswers[index], index);
      if (!parsed) return setError("Include the subject name, HL or SL, and a current grade from 1 to 7. Example: Physics HL, currently 5.");
      const nextSubjects = subjects.map((subject, subjectIndex) => subjectIndex === index ? parsed : subject);
      if (index === 5) {
        const nextHlCount = nextSubjects.filter((subject) => subject.level === "HL").length;
        if (nextHlCount < 3 || nextHlCount > 4) return setError(`I counted ${nextHlCount} HL subjects. The diploma needs three or four—go back and correct the level on one answer.`);
      }
      setSubjects(nextSubjects);
    }
    goForward();
  }

  async function finish() {
    setError("");
    if (hlCount < 3 || hlCount > 4 || subjects.some((subject) => !subject.name)) {
      setError("Your subject map needs six named subjects and three or four HL choices.");
      return;
    }
    try {
      await onComplete({ ...profile, subjects, ...core });
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }

  function jumpTo(index: number) {
    setReviewing(false);
    setQuestionIndex(index);
    setError("");
  }

  return (
    <main className="onboarding-shell">
      <section className="onboarding-aside">
        <div className="brand-lockup brand-lockup--light">
          <span className="brand-mark">45</span>
          <span><strong>IB 45</strong><small>Coach</small></span>
        </div>
        <div className="onboarding-promise">
          <p className="eyebrow eyebrow--light">A guided conversation</p>
          <h1>You answer. I build the system.</h1>
          <p>No setup spreadsheet and no giant form. I will ask what matters, translate it into your academic profile, and show you everything before saving.</p>
        </div>
        <div className="privacy-note"><ShieldCheck size={18} /><span>Your answers and academic records stay encrypted on drive D.</span></div>
      </section>

      <section className="onboarding-main">
        <div className="onboarding-card interview-card">
          <header className="interview-progress" aria-label={`Setup ${progress}% complete`}>
            <div><span>{reviewing ? "Review" : current.stage}</span><strong>{progress}%</strong></div>
            <div className="progress-track"><span style={{ width: `${progress}%` }} /></div>
          </header>

          {reviewing ? (
            <Review profile={profile} subjects={subjects} core={core} onEdit={jumpTo} />
          ) : (
            <section className="interview-question" aria-live="polite">
              <div className="coach-bubble"><span>45</span><p>{current.title}</p></div>
              <p className="interview-help">{current.help}</p>
              <QuestionAnswer
                question={current}
                profile={profile}
                setProfile={setProfile}
                capacityHours={capacityHours}
                setCapacityHours={setCapacityHours}
                subjectAnswers={subjectAnswers}
                setSubjectAnswers={setSubjectAnswers}
                core={core}
                setCore={setCore}
                choose={choose}
                submitText={submitText}
                inputRef={inputRef}
              />
            </section>
          )}

          <div className="onboarding-actions interview-actions">
            <button className="button button--ghost" type="button" disabled={(!reviewing && questionIndex === 0 && !onCancel) || busy} onClick={goBack}><ArrowLeft size={16} /> {!reviewing && questionIndex === 0 && onCancel ? "Exit interview" : "Back"}</button>
            <span className="form-error" role="alert">{error}</span>
            {reviewing ? (
              <button className="button button--primary" type="button" disabled={busy} onClick={finish}>{busy ? "Building your coach…" : "Everything looks right"}<Check size={16} /></button>
            ) : isTextQuestion(current.id) ? (
              <button className="button button--primary" type="button" disabled={busy} onClick={submitText}>Answer<ArrowRight size={16} /></button>
            ) : <span className="answer-hint">Choose an answer to continue</span>}
          </div>
        </div>
      </section>
    </main>
  );
}

type AnswerProps = {
  question: Question;
  profile: ProfileDraft;
  setProfile: React.Dispatch<React.SetStateAction<ProfileDraft>>;
  capacityHours: string;
  setCapacityHours: (value: string) => void;
  subjectAnswers: string[];
  setSubjectAnswers: React.Dispatch<React.SetStateAction<string[]>>;
  core: { tokGrade: string; eeGrade: string; casComplete: boolean };
  setCore: React.Dispatch<React.SetStateAction<{ tokGrade: string; eeGrade: string; casComplete: boolean }>>;
  choose: (update: () => void) => void;
  submitText: () => void;
  inputRef: React.RefObject<HTMLInputElement | HTMLTextAreaElement | null>;
};

function QuestionAnswer({ question, profile, setProfile, capacityHours, setCapacityHours, subjectAnswers, setSubjectAnswers, core, setCore, choose, submitText, inputRef }: AnswerProps) {
  const subjectIndex = question.id.startsWith("subject") ? Number(question.id.replace("subject", "")) : -1;
  const onEnter = (event: React.KeyboardEvent<HTMLInputElement>) => { if (event.key === "Enter") submitText(); };

  if (question.id === "name") return <div className="interview-answer"><input ref={inputRef as React.RefObject<HTMLInputElement>} className="interview-input" value={profile.name} onChange={(event) => setProfile({ ...profile, name: event.target.value })} onKeyDown={onEnter} placeholder={question.placeholder} aria-label="Your name" /></div>;
  if (question.id === "capacity") return <div className="interview-answer"><div className="interview-number"><input ref={inputRef as React.RefObject<HTMLInputElement>} className="interview-input" type="number" min="3" max="40" value={capacityHours} onChange={(event) => setCapacityHours(event.target.value)} onKeyDown={onEnter} aria-label="Focused study hours each week" /><span>hours / week</span></div><div className="quick-options">{[8, 12, 16, 20].map((hours) => <button type="button" key={hours} onClick={() => setCapacityHours(String(hours))}>{hours} hours</button>)}</div></div>;
  if (subjectIndex >= 0) return <div className="interview-answer"><input ref={inputRef as React.RefObject<HTMLInputElement>} className="interview-input" value={subjectAnswers[subjectIndex]} onChange={(event) => setSubjectAnswers((answers) => answers.map((answer, index) => index === subjectIndex ? event.target.value : answer))} onKeyDown={onEnter} placeholder={question.placeholder} aria-label={`Subject ${subjectIndex + 1}, level and current grade`} /><div className="parse-example"><Sparkles size={15} /><span>I will separate the subject, level and grade for you.</span></div></div>;
  if (question.id === "policy") return <div className="interview-answer"><textarea ref={inputRef as React.RefObject<HTMLTextAreaElement>} className="interview-input interview-textarea" rows={5} value={profile.schoolAiPolicy} onChange={(event) => setProfile({ ...profile, schoolAiPolicy: event.target.value })} placeholder={question.placeholder} aria-label="School AI policy" /><button className="skip-answer" type="button" onClick={() => choose(() => setProfile({ ...profile, schoolAiPolicy: "Policy not entered yet; confirm the school's rules before assessed-work assistance." }))}>I don’t know yet—ask me later</button></div>;
  if (question.id === "exam") return <ChoiceGrid options={["May 2027", "November 2027", "May 2028", "November 2028"]} selected={profile.examSession} onChoose={(value) => choose(() => setProfile({ ...profile, examSession: value }))} />;
  if (question.id === "sleepStart" || question.id === "sleepEnd") return <ChoiceGrid icon={<Clock3 size={16} />} options={timeOptions[question.id].map((value) => ({ value, label: formatTime(value) }))} selected={profile[question.id]} onChoose={(value) => choose(() => setProfile({ ...profile, [question.id]: value }))} />;
  if (question.id === "tok" || question.id === "ee") {
    const field = question.id === "tok" ? "tokGrade" : "eeGrade";
    return <ChoiceGrid options={["A", "B", "C", "D", "E", "Not graded yet"]} selected={core[field]} onChoose={(value) => choose(() => setCore({ ...core, [field]: value === "Not graded yet" ? "C" : value }))} />;
  }
  return <ChoiceGrid options={[{ value: "yes", label: "Yes, complete" }, { value: "no", label: "No, still in progress" }]} selected={core.casComplete ? "yes" : "no"} onChoose={(value) => choose(() => setCore({ ...core, casComplete: value === "yes" }))} />;
}

function ChoiceGrid({ options, selected, onChoose, icon }: { options: Array<string | { value: string; label: string }>; selected: string; onChoose: (value: string) => void; icon?: React.ReactNode }) {
  return <div className="interview-choices">{options.map((option) => { const value = typeof option === "string" ? option : option.value; const label = typeof option === "string" ? option : option.label; return <button className={selected === value ? "is-selected" : ""} type="button" key={value} onClick={() => onChoose(value)}>{icon}{label}{selected === value && <Check size={16} />}</button>; })}</div>;
}

function Review({ profile, subjects, core, onEdit }: { profile: ProfileDraft; subjects: SubjectDraft[]; core: { tokGrade: string; eeGrade: string; casComplete: boolean }; onEdit: (index: number) => void }) {
  return <section className="interview-review">
    <p className="eyebrow">One last check</p>
    <h2>I turned your answers into this starting profile.</h2>
    <p className="muted">Nothing has been saved yet. Edit any answer that does not look right.</p>
    <div className="review-summary">
      <article><div><span>Student and schedule</span><button type="button" onClick={() => onEdit(0)}><Pencil size={14} /> Edit</button></div><strong>{profile.name} · {profile.examSession}</strong><p>{Math.round(profile.weeklyCapacityMinutes / 60)} focused hours/week · sleep {formatTime(profile.sleepStart)}–{formatTime(profile.sleepEnd)}</p></article>
      <article><div><span>Six subjects</span><button type="button" onClick={() => onEdit(5)}><Pencil size={14} /> Edit</button></div><div className="review-subjects">{subjects.map((subject) => <span key={`${subject.groupNumber}-${subject.name}`}><strong>{subject.name}</strong><small>{subject.level} · now {subject.currentGrade} → target 7</small></span>)}</div></article>
      <article><div><span>Diploma core</span><button type="button" onClick={() => onEdit(11)}><Pencil size={14} /> Edit</button></div><strong>TOK {core.tokGrade} · EE {core.eeGrade} · CAS {core.casComplete ? "complete" : "in progress"}</strong></article>
    </div>
    <div className="integrity-callout"><ShieldCheck size={18} /><p><strong>Private and reversible.</strong> The coach stores this in the encrypted database on D. You can update every value later.</p></div>
  </section>;
}

export function parseSubjectAnswer(raw: string, index: number): SubjectDraft | null {
  const level = raw.match(/\b(HL|SL)\b/i)?.[1]?.toUpperCase() as "HL" | "SL" | undefined;
  const gradeMatches = [...raw.matchAll(/\b([1-7])\b/g)];
  const grade = gradeMatches.length ? Number(gradeMatches[gradeMatches.length - 1][1]) : NaN;
  const name = raw
    .replace(/\b(HL|SL)\b/ig, " ")
    .replace(/\b(?:currently|current(?:ly)?|grade|now|getting|at)\s*[:=\-]?\s*[1-7]\b/ig, " ")
    .replace(/[,;:/\-]?\s*[1-7]\s*$/g, " ")
    .replace(/\s+/g, " ")
    .replace(/^[,;:\-\s]+|[,;:\-\s]+$/g, "")
    .trim();
  if (!name || !level || !Number.isFinite(grade)) return null;
  return { name, level, groupNumber: index + 1, syllabusVersion: "Current", currentGrade: grade, targetGrade: 7 };
}

function isTextQuestion(id: QuestionId) {
  return id === "name" || id === "capacity" || id.startsWith("subject") || id === "policy";
}

function formatTime(value: string) {
  const [hourText, minute] = value.split(":");
  const hour = Number(hourText);
  const suffix = hour >= 12 ? "pm" : "am";
  const display = hour % 12 || 12;
  return `${display}:${minute} ${suffix}`;
}
