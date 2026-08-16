import { FormEvent, useDeferredValue, useEffect, useMemo, useState } from "react";
import {
  AlertTriangle, ArrowRight, BookOpen, BrainCircuit, CalendarDays, Check,
  CheckCircle2, ChevronRight, CircleGauge, Clock3, Cloud, Database, FileSearch,
  FilePenLine, GraduationCap, HardDrive, KeyRound, LayoutDashboard, ListTodo, LoaderCircle,
  Menu, MessageCircleQuestion, Minus, Pause, Play, Plus, RefreshCw, Search, Settings, ShieldCheck, Sparkles,
  Target, TimerReset, Upload, WifiOff, X,
} from "lucide-react";
import { Area, AreaChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts";
import { format, formatDistanceToNow, parseISO } from "date-fns";
import { api, call } from "./api";
import { Onboarding } from "./Onboarding";
import { QuickCapture } from "./QuickCapture";
import { ExamLab } from "./ExamLab";
import { adjacentDisplayScale, applyDisplayScale, DISPLAY_SCALE_OPTIONS, readDisplayScale, type DisplayScale } from "./displayScale";
import type {
  AiAnalysis, AssessmentRecord, CalendarStatus, CoreProgress, DashboardSnapshot,
  IndexStatus, ResourceResult, SecretStatus, Subject, Task, ViewId,
} from "./types";
import "./App.css";

const navItems: { id: ViewId; label: string; icon: typeof LayoutDashboard }[] = [
  { id: "dashboard", label: "Command center", icon: LayoutDashboard },
  { id: "subjects", label: "Subjects", icon: BookOpen },
  { id: "plan", label: "Action plan", icon: ListTodo },
  { id: "resources", label: "Resource library", icon: FileSearch },
  { id: "exam", label: "Past Paper Lab", icon: FilePenLine },
  { id: "core", label: "TOK · EE · CAS", icon: GraduationCap },
  { id: "coach", label: "AI coach", icon: BrainCircuit },
  { id: "calendar", label: "Calendar", icon: CalendarDays },
];

const viewTitles: Record<ViewId, { title: string; subtitle: string }> = {
  dashboard: { title: "Command center", subtitle: "The clearest next move, grounded in your evidence." },
  subjects: { title: "Subjects", subtitle: "Turn every score and mistake into a more accurate projection." },
  plan: { title: "Action plan", subtitle: "Prioritized by urgency, recurring weakness and expected impact." },
  resources: { title: "Resource library", subtitle: "Search every relevant IB file on drive D without changing the source." },
  exam: { title: "Past Paper Lab", subtitle: "Timed papers, editable PDF answers and evidence-backed scoring." },
  core: { title: "TOK · EE · CAS", subtitle: "Protect the three core points—and diploma eligibility." },
  coach: { title: "AI coach", subtitle: "Ask for analysis, scenarios or a realistic revision strategy." },
  calendar: { title: "Calendar", subtitle: "Fit the plan around the life that is already scheduled." },
  settings: { title: "Settings & privacy", subtitle: "Control providers, credentials, backups and notifications." },
};

function App() {
  const [snapshot, setSnapshot] = useState<DashboardSnapshot>();
  const [view, setView] = useState<ViewId>("dashboard");
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [guidedSetup, setGuidedSetup] = useState(false);
  const [quickCaptureOpen, setQuickCaptureOpen] = useState(false);
  const [displayScale, setDisplayScale] = useState<DisplayScale>(() => readDisplayScale());
  const [toast, setToast] = useState<{ tone: "success" | "error"; message: string }>();

  useEffect(() => {
    api.initialize().then(setSnapshot).catch((error) => notify("error", readableError(error))).finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => setToast(undefined), 4500);
    return () => window.clearTimeout(timer);
  }, [toast]);

  useEffect(() => {
    void applyDisplayScale(displayScale);
  }, [displayScale]);

  function notify(tone: "success" | "error", message: string) { setToast({ tone, message }); }

  async function refresh() {
    const next = await api.refresh();
    setSnapshot(next);
    return next;
  }

  async function onboard(input: Record<string, unknown>) {
    setBusy(true);
    try {
      setSnapshot(await api.onboard(input));
      notify("success", "Your private command center is ready.");
    } catch (error) {
      notify("error", readableError(error));
      throw error;
    } finally { setBusy(false); }
  }

  async function updateFromInterview(input: Record<string, unknown>) {
    await onboard(input);
    setGuidedSetup(false);
  }

  if (loading) return <LoadingScreen />;
  if (!snapshot?.onboarded || guidedSetup) return <Onboarding busy={busy} onComplete={guidedSetup ? updateFromInterview : onboard} initial={guidedSetup ? snapshot : undefined} onCancel={guidedSetup ? () => setGuidedSetup(false) : undefined} />;

  const title = viewTitles[view];
  return (
    <div className="app-shell">
      <aside className={`sidebar ${sidebarOpen ? "sidebar--open" : ""}`}>
        <div className="brand-lockup"><span className="brand-mark">45</span><span><strong>IB 45</strong><small>Coach</small></span></div>
        <nav className="sidebar-nav" aria-label="Primary navigation">
          <p className="nav-label">Workspace</p>
          {navItems.map((item) => <NavButton key={item.id} item={item} active={view === item.id} onClick={() => { setView(item.id); setSidebarOpen(false); }} />)}
          <p className="nav-label nav-label--second">System</p>
          <NavButton item={{ id: "settings", label: "Settings & privacy", icon: Settings }} active={view === "settings"} onClick={() => { setView("settings"); setSidebarOpen(false); }} />
        </nav>
        <div className="sidebar-footer">
          <div className="privacy-pill"><ShieldCheck size={16} /><span><strong>Local-first</strong><small>Data rooted on D:</small></span></div>
          <div className="profile-pill"><span className="avatar">{snapshot.profile?.name.slice(0, 1).toUpperCase()}</span><span><strong>{snapshot.profile?.name}</strong><small>{snapshot.profile?.examSession}</small></span></div>
        </div>
      </aside>

      <section className="app-main">
        <header className="topbar">
          <button className="icon-button mobile-menu" onClick={() => setSidebarOpen((value) => !value)} aria-label="Open navigation"><Menu size={20} /></button>
          <div><p className="eyebrow">{snapshot.profile?.examSession} · {snapshot.profile?.timezone}</p><h1>{title.title}</h1><p>{title.subtitle}</p></div>
          <div className="topbar-actions">
            <span className="sync-badge"><span className="status-dot" /> Private workspace</span>
            <div className="scale-stepper" role="group" aria-label="Interface scale">
              <button onClick={() => setDisplayScale((scale) => adjacentDisplayScale(scale, -1))} disabled={displayScale === DISPLAY_SCALE_OPTIONS[0].value} aria-label="Make interface smaller" title="Make interface smaller"><Minus size={14} /></button>
              <span aria-live="polite">{Math.round(displayScale * 100)}%</span>
              <button onClick={() => setDisplayScale((scale) => adjacentDisplayScale(scale, 1))} disabled={displayScale === DISPLAY_SCALE_OPTIONS[DISPLAY_SCALE_OPTIONS.length - 1].value} aria-label="Make interface larger" title="Make interface larger"><Plus size={14} /></button>
            </div>
            <button className="button button--quiet quick-capture-trigger" onClick={() => setQuickCaptureOpen(true)}><MessageCircleQuestion size={16} /> Quick capture</button>
            {view !== "plan" && <button className="button button--primary" onClick={() => setView("plan")}><Plus size={16} /> Add action</button>}
          </div>
        </header>

        <main className="content" id="main-content">
          {view === "dashboard" && <Dashboard snapshot={snapshot} setView={setView} onComplete={async (task) => { await completeTask(task, notify); await refresh(); }} />}
          {view === "subjects" && <SubjectsView snapshot={snapshot} notify={notify} refresh={refresh} />}
          {view === "plan" && <PlanView snapshot={snapshot} notify={notify} refresh={refresh} />}
          {view === "resources" && <ResourcesView snapshot={snapshot} notify={notify} refresh={refresh} />}
          {view === "exam" && <ExamLab subjects={snapshot.subjects} notify={notify} />}
          {view === "core" && <CoreView core={snapshot.core} notify={notify} refresh={refresh} />}
          {view === "coach" && <CoachView snapshot={snapshot} notify={notify} />}
          {view === "calendar" && <CalendarView notify={notify} />}
          {view === "settings" && <div className="settings-layout"><DisplayScaleSettings value={displayScale} onChange={setDisplayScale} /><SettingsView notify={notify} onGuidedSetup={() => setGuidedSetup(true)} /></div>}
        </main>
      </section>
      {sidebarOpen && <button className="sidebar-scrim" aria-label="Close navigation" onClick={() => setSidebarOpen(false)} />}
      {quickCaptureOpen && <QuickCapture subjects={snapshot.subjects} onClose={() => setQuickCaptureOpen(false)} onSaved={async (message) => { await refresh(); notify("success", message); }} />}
      {toast && <div className={`toast toast--${toast.tone}`} role="status">{toast.tone === "success" ? <CheckCircle2 size={18} /> : <AlertTriangle size={18} />}<span>{toast.message}</span><button onClick={() => setToast(undefined)} aria-label="Dismiss"><X size={15} /></button></div>}
    </div>
  );
}

function NavButton({ item, active, onClick }: { item: { id: ViewId; label: string; icon: typeof LayoutDashboard }; active: boolean; onClick: () => void }) {
  const Icon = item.icon;
  return <button className={`nav-item ${active ? "nav-item--active" : ""}`} onClick={onClick}><Icon size={18} /><span>{item.label}</span>{active && <span className="nav-active-dot" />}</button>;
}

function LoadingScreen() {
  return <main className="loading-screen"><div className="loading-mark">45</div><LoaderCircle className="spin" size={22} /><p>Opening your private command center…</p></main>;
}

function Dashboard({ snapshot, setView, onComplete }: { snapshot: DashboardSnapshot; setView: (view: ViewId) => void; onComplete: (task: Task) => Promise<void> }) {
  const trend = useMemo(() => {
    const current = snapshot.projection.totalPoints;
    return [{ label: "Baseline", value: Math.max(24, current - 4) }, { label: "Earlier", value: Math.max(24, current - 2) }, { label: "Now", value: current }, { label: "High", value: snapshot.projection.high }];
  }, [snapshot]);
  const nextTask = snapshot.tasks[0];
  return <div className="dashboard-grid">
    <section className="hero-score-card">
      <div className="hero-score-main">
        <div><p className="eyebrow eyebrow--light">Evidence-calibrated projection</p><div className="score-lockup"><strong>{snapshot.projection.totalPoints}</strong><span>/45</span></div><p className="confidence-range">Likely range {snapshot.projection.low}–{snapshot.projection.high} · {Math.round(snapshot.projection.confidence * 100)}% confidence</p></div>
        <div className="trajectory-ring" style={{ "--progress": `${snapshot.projection.totalPoints / 45 * 360}deg` } as React.CSSProperties}><span>{snapshot.projection.targetGap}</span><small>point gap</small></div>
      </div>
      <div className="hero-chart"><ResponsiveContainer width="100%" height={112}><AreaChart data={trend}><defs><linearGradient id="scoreFill" x1="0" y1="0" x2="0" y2="1"><stop offset="0%" stopColor="#9bbcff" stopOpacity={0.55}/><stop offset="100%" stopColor="#9bbcff" stopOpacity={0}/></linearGradient></defs><XAxis dataKey="label" axisLine={false} tickLine={false} tick={{ fill: "#c7d5ef", fontSize: 11 }} /><YAxis hide domain={[20,45]} /><Tooltip contentStyle={{ borderRadius: 10, border: "none", background: "#fff" }} /><Area type="monotone" dataKey="value" stroke="#d7e5ff" strokeWidth={2.5} fill="url(#scoreFill)" /></AreaChart></ResponsiveContainer></div>
      <div className="hero-score-footer"><span><Target size={15} /> 42 subject points + 3 core points</span><button onClick={() => setView("subjects")}>Inspect projection <ArrowRight size={15} /></button></div>
    </section>

    <section className="card next-action-card">
      <div className="card-header"><div><p className="eyebrow">Highest-leverage move</p><h2>Do this next</h2></div><span className="priority-badge">P1</span></div>
      {nextTask ? <><div className="next-task-subject">{subjectName(snapshot.subjects, nextTask.subjectId)}</div><h3>{nextTask.title}</h3><p>{nextTask.rationale}</p><div className="task-facts"><span><Clock3 size={15} /> {nextTask.effortMinutes} min</span><span><CircleGauge size={15} /> Impact {nextTask.expectedImpact.toFixed(1)}</span></div><div className="next-task-actions"><button className="button button--primary" onClick={() => onComplete(nextTask)}><Check size={16} /> Mark complete</button><button className="button button--quiet" onClick={() => setView("plan")}>Open plan</button></div></> : <EmptyState icon={CheckCircle2} title="The queue is clear" body="Add your next assessed deadline or practice block." />}
    </section>

    <section className="card subject-performance-card">
      <div className="card-header"><div><p className="eyebrow">Six-subject map</p><h2>Performance vs 7</h2></div><button className="text-button" onClick={() => setView("subjects")}>All subjects <ChevronRight size={15} /></button></div>
      <div className="subject-bars">{snapshot.subjects.map((subject) => <div className="subject-bar-row" key={subject.id}><div className="subject-label"><span className="color-dot" style={{ background: subject.accent }} /><span><strong>{subject.name}</strong><small>{subject.level} · {Math.round(subject.confidence * 100)}% confidence</small></span></div><div className="bullet-track" aria-label={`${subject.name}: ${subject.currentGrade} out of 7`}><span style={{ width: `${subject.currentGrade / 7 * 100}%`, background: subject.accent }} /><i style={{ left: `${subject.targetGrade / 7 * 100}%` }} /></div><strong className="grade-value">{subject.currentGrade}</strong></div>)}</div>
      <div className="chart-legend"><span><i className="legend-line" /> Current projection</span><span><i className="legend-target" /> Target</span></div>
    </section>

    <section className="card core-card">
      <div className="card-header"><div><p className="eyebrow">Diploma core</p><h2>{snapshot.core.corePoints}/3 core points</h2></div><button className="icon-button" onClick={() => setView("core")} aria-label="Open core tracker"><ChevronRight size={18} /></button></div>
      <div className="core-metrics"><div><span>TOK</span><strong>{snapshot.core.tokGrade}</strong><small>Working grade</small></div><div><span>EE</span><strong>{snapshot.core.eeGrade}</strong><small>{snapshot.core.eeWordCount.toLocaleString()} words</small></div><div className={snapshot.core.casComplete ? "core-ok" : "core-risk"}><span>CAS</span><strong>{snapshot.core.casComplete ? "Met" : "Risk"}</strong><small>{snapshot.core.casReflections} reflections</small></div></div>
      {!snapshot.core.casComplete && <div className="risk-callout"><AlertTriangle size={16} /><span>CAS is ungraded, but incomplete CAS prevents the diploma.</span></div>}
    </section>

    <section className="card plan-preview-card">
      <div className="card-header"><div><p className="eyebrow">Accountability queue</p><h2>Next actions</h2></div><span className="count-chip">{snapshot.tasks.length} open</span></div>
      <div className="compact-task-list">{snapshot.tasks.slice(0,4).map((task, index) => <button key={task.id} onClick={() => setView("plan")}><span className="task-rank">{index + 1}</span><span><strong>{task.title}</strong><small>{formatDue(task.dueAt)} · {task.effortMinutes} min</small></span><span className="impact-score">{Math.round(task.priorityScore)}</span></button>)}</div>
    </section>

    <section className="card library-snapshot-card">
      <div className="card-header"><div><p className="eyebrow">Local intelligence</p><h2>Resource index</h2></div><HardDrive size={19} /></div>
      <div className="library-number">{snapshot.indexedCount.toLocaleString()}<span> / {snapshot.resourceCount.toLocaleString()} files</span></div>
      <div className="progress-track"><span style={{ width: `${snapshot.resourceCount ? snapshot.indexedCount / snapshot.resourceCount * 100 : 0}%` }} /></div>
      <button className="button button--quiet button--wide" onClick={() => setView("resources")}><Search size={16} /> Search library</button>
    </section>
  </div>;
}

function SubjectsView({ snapshot, notify, refresh }: { snapshot: DashboardSnapshot; notify: Notify; refresh: () => Promise<DashboardSnapshot> }) {
  const [selectedId, setSelectedId] = useState(snapshot.subjects[0]?.id);
  const [assessments, setAssessments] = useState<AssessmentRecord[]>([]);
  const [showForm, setShowForm] = useState(false);
  const selected = snapshot.subjects.find((subject) => subject.id === selectedId) ?? snapshot.subjects[0];
  useEffect(() => {
    if (!selectedId) return;
    call<AssessmentRecord[]>("get_assessments", { subjectId: selectedId }, []).then(setAssessments).catch((error) => notify("error", readableError(error)));
  }, [notify, selectedId]);
  if (!selected) return <EmptyState icon={BookOpen} title="No subjects yet" body="Complete onboarding to add the six IB subjects." />;
  return <div className="split-workspace">
    <section className="subject-list-panel"><div className="panel-heading"><p className="eyebrow">Your diploma</p><h2>Six subjects</h2></div>{snapshot.subjects.map((subject) => <button className={`subject-list-item ${subject.id === selected.id ? "is-active" : ""}`} onClick={() => setSelectedId(subject.id)} key={subject.id}><span className="subject-icon" style={{ background: `${subject.accent}18`, color: subject.accent }}>{subject.name.slice(0,2).toUpperCase()}</span><span><strong>{subject.name}</strong><small>Group {subject.groupNumber} · {subject.level}</small></span><span className="subject-grade">{subject.currentGrade}<small>/7</small></span></button>)}</section>
    <section className="subject-detail-panel">
      <div className="detail-hero" style={{ "--subject-accent": selected.accent } as React.CSSProperties}><div><p className="eyebrow">Group {selected.groupNumber} · {selected.level}</p><h2>{selected.name}</h2><p>{selected.syllabusVersion} syllabus</p></div><div className="detail-grade"><strong>{selected.currentGrade}</strong><span>Current projection</span><small>{Math.round(selected.confidence * 100)}% confidence</small></div></div>
      <div className="stat-strip"><div><span>Target</span><strong>{selected.targetGrade}/7</strong></div><div><span>Gap</span><strong>{selected.targetGrade - selected.currentGrade} point{selected.targetGrade - selected.currentGrade === 1 ? "" : "s"}</strong></div><div><span>Evidence</span><strong>{assessments.length} records</strong></div><button className="button button--primary" onClick={() => setShowForm((value) => !value)}><Plus size={16} /> Add evidence</button></div>
      {showForm && <AssessmentForm subject={selected} onSaved={async () => { setShowForm(false); setAssessments(await call("get_assessments", { subjectId: selected.id }, [])); await refresh(); notify("success", "Assessment saved and the projection recalibrated."); }} notify={notify} />}
      <div className="section-heading"><div><p className="eyebrow">Assessment history</p><h3>What the evidence says</h3></div></div>
      {assessments.length ? <div className="assessment-list">{assessments.map((assessment) => <article key={assessment.id}><div className="assessment-score"><strong>{Math.round(assessment.percentage)}%</strong><small>{assessment.ibGrade ? `IB ${assessment.ibGrade}` : "Provisional"}</small></div><div><div className="assessment-title"><strong>{assessment.title}</strong><span>{assessment.component}</span></div><p>{assessment.whyLostMarks || assessment.feedback || "No diagnosis recorded yet."}</p><div className="tag-row">{assessment.errorCategories.map((category) => <span key={category}>{category}</span>)}</div></div><time>{formatShortDate(assessment.occurredAt)}</time></article>)}</div> : <EmptyState icon={Upload} title="No evidence recorded" body="Add a test, paper, IA checkpoint or teacher score to calibrate this subject." />}
    </section>
  </div>;
}

function AssessmentForm({ subject, onSaved, notify }: { subject: Subject; onSaved: () => Promise<void>; notify: Notify }) {
  const errorOptions = ["Knowledge gap", "Interpretation", "Method", "Evidence", "Structure", "Terminology", "Time management", "Careless execution"];
  const [form, setForm] = useState({ title: "", assessmentType: "Test", component: "Paper 1", score: 0, maxScore: 100, weight: 1, ibGrade: 0, occurredAt: new Date().toISOString().slice(0,10), feedback: "", whyLostMarks: "", errorCategories: [] as string[] });
  const [saving, setSaving] = useState(false);
  async function submit(event: FormEvent) {
    event.preventDefault(); setSaving(true);
    try {
      await call("add_assessment", { input: { ...form, subjectId: subject.id, occurredAt: new Date(`${form.occurredAt}T12:00:00`).toISOString(), ibGrade: form.ibGrade || null, attachmentPath: null } });
      await onSaved();
    } catch (error) { notify("error", readableError(error)); } finally { setSaving(false); }
  }
  return <form className="inline-form" onSubmit={submit}><div className="inline-form-header"><div><p className="eyebrow">New evidence</p><h3>Record an assessment</h3></div><span>{subject.name}</span></div><div className="form-grid form-grid--three"><label className="field field--span-two"><span>Assessment title *</span><input required value={form.title} onChange={(event) => setForm({ ...form, title: event.target.value })} placeholder="e.g. Topic 8 test" /></label><label className="field"><span>Date</span><input type="date" value={form.occurredAt} onChange={(event) => setForm({ ...form, occurredAt: event.target.value })} /></label><label className="field"><span>Component</span><input value={form.component} onChange={(event) => setForm({ ...form, component: event.target.value })} /></label><label className="field"><span>Score</span><input type="number" min={0} value={form.score} onChange={(event) => setForm({ ...form, score: Number(event.target.value) })} /></label><label className="field"><span>Out of</span><input type="number" min={1} value={form.maxScore} onChange={(event) => setForm({ ...form, maxScore: Number(event.target.value) })} /></label><label className="field"><span>Teacher IB grade (optional)</span><select value={form.ibGrade} onChange={(event) => setForm({ ...form, ibGrade: Number(event.target.value) })}><option value={0}>Not provided</option>{[1,2,3,4,5,6,7].map((grade) => <option value={grade} key={grade}>{grade}</option>)}</select></label><label className="field field--span-two"><span>Why were marks lost?</span><textarea rows={3} value={form.whyLostMarks} onChange={(event) => setForm({ ...form, whyLostMarks: event.target.value })} /></label></div><fieldset className="tag-selector"><legend>Error categories</legend>{errorOptions.map((option) => <button type="button" className={form.errorCategories.includes(option) ? "is-selected" : ""} onClick={() => setForm({ ...form, errorCategories: form.errorCategories.includes(option) ? form.errorCategories.filter((value) => value !== option) : [...form.errorCategories, option] })} key={option}>{option}</button>)}</fieldset><div className="form-actions"><button className="button button--primary" disabled={saving}>{saving ? "Saving…" : "Save evidence"}</button></div></form>;
}

function PlanView({ snapshot, notify, refresh }: { snapshot: DashboardSnapshot; notify: Notify; refresh: () => Promise<DashboardSnapshot> }) {
  const [showForm, setShowForm] = useState(false);
  const [tasks, setTasks] = useState(snapshot.tasks);
  useEffect(() => { setTasks(snapshot.tasks); }, [snapshot.tasks]);
  async function mark(task: Task) { await completeTask(task, notify); const next = await refresh(); setTasks(next.tasks); }
  return <div className="plan-layout"><section className="plan-main"><div className="section-heading"><div><p className="eyebrow">Adaptive queue</p><h2>Ordered by leverage—not anxiety</h2><p>Missed work is rescheduled around remaining capacity instead of being stacked unrealistically.</p></div><button className="button button--primary" onClick={() => setShowForm((value) => !value)}><Plus size={16} /> New action</button></div>{showForm && <TaskForm subjects={snapshot.subjects} notify={notify} onSaved={async () => { setShowForm(false); const next = await refresh(); setTasks(next.tasks); }} />}<div className="full-task-list">{tasks.map((task, index) => <article key={task.id}><button className="complete-button" onClick={() => mark(task)} aria-label={`Complete ${task.title}`}><Check size={17} /></button><div className="task-order">{String(index + 1).padStart(2,"0")}</div><div className="task-content"><div className="task-title-row"><strong>{task.title}</strong><span>{subjectName(snapshot.subjects, task.subjectId)}</span></div><p>{task.rationale}</p><div className="task-meta"><span><Clock3 size={14} /> {task.effortMinutes} min</span><span><Target size={14} /> Impact {task.expectedImpact.toFixed(1)}</span><span className={isOverdue(task.dueAt) ? "overdue" : ""}><CalendarDays size={14} /> {formatDue(task.dueAt)}</span></div><div className="evidence-line"><ShieldCheck size={14} /> Evidence: {task.evidenceRequirement || "Completion check-in"}</div></div><div className="priority-meter"><strong>{Math.round(task.priorityScore)}</strong><small>priority</small><span style={{ height: `${Math.min(100, task.priorityScore)}%` }} /></div></article>)}</div>{!tasks.length && <EmptyState icon={CheckCircle2} title="Nothing is waiting" body="Create the next smallest action that would produce useful evidence." />}</section><aside className="plan-aside"><div className="card capacity-card"><p className="eyebrow">Protected capacity</p><h3>{Math.round((snapshot.profile?.weeklyCapacityMinutes ?? 0) / 60)} hours/week</h3><div className="capacity-grid"><span><strong>{snapshot.tasks.reduce((sum, task) => sum + task.effortMinutes, 0)}</strong><small>minutes queued</small></span><span><strong>{snapshot.overdueCount}</strong><small>overdue</small></span></div><div className="risk-callout risk-callout--neutral"><TimerReset size={16} /><span>Sleep {snapshot.profile?.sleepStart}–{snapshot.profile?.sleepEnd} stays protected.</span></div></div><div className="card"><p className="eyebrow">Planning rule</p><h3>Expected gain per hour</h3><p className="muted">Deadline risk, recurring weaknesses and shorter high-impact work rise first. The AI can suggest actions, but deterministic policy sets the queue.</p></div></aside></div>;
}

function TaskForm({ subjects, notify, onSaved }: { subjects: Subject[]; notify: Notify; onSaved: () => Promise<void> }) {
  const [form, setForm] = useState({ subjectId: subjects[0]?.id ?? "", title: "", rationale: "", dueAt: new Date(Date.now()+86_400_000).toISOString().slice(0,16), effortMinutes: 45, expectedImpact: 0.5, evidenceRequirement: "" });
  const [saving, setSaving] = useState(false);
  async function submit(event: FormEvent) { event.preventDefault(); setSaving(true); try { await call("create_task", { input: { ...form, subjectId: form.subjectId || null, dueAt: new Date(form.dueAt).toISOString() } }); await onSaved(); notify("success", "Action added to the adaptive queue."); } catch (error) { notify("error", readableError(error)); } finally { setSaving(false); } }
  return <form className="inline-form" onSubmit={submit}><div className="inline-form-header"><div><p className="eyebrow">New action</p><h3>Make it specific and provable</h3></div></div><div className="form-grid form-grid--three"><label className="field"><span>Subject</span><select value={form.subjectId} onChange={(event) => setForm({ ...form, subjectId: event.target.value })}><option value="">Core / general</option>{subjects.map((subject) => <option value={subject.id} key={subject.id}>{subject.name}</option>)}</select></label><label className="field field--span-two"><span>Action *</span><input required value={form.title} onChange={(event) => setForm({ ...form, title: event.target.value })} placeholder="Start with a verb" /></label><label className="field field--span-two"><span>Why this matters</span><input value={form.rationale} onChange={(event) => setForm({ ...form, rationale: event.target.value })} /></label><label className="field"><span>Due</span><input type="datetime-local" value={form.dueAt} onChange={(event) => setForm({ ...form, dueAt: event.target.value })} /></label><label className="field"><span>Effort (minutes)</span><input type="number" min={15} step={5} value={form.effortMinutes} onChange={(event) => setForm({ ...form, effortMinutes: Number(event.target.value) })} /></label><label className="field"><span>Expected point impact</span><input type="number" min={0.1} max={3} step={0.1} value={form.expectedImpact} onChange={(event) => setForm({ ...form, expectedImpact: Number(event.target.value) })} /></label><label className="field"><span>Proof of completion</span><input value={form.evidenceRequirement} onChange={(event) => setForm({ ...form, evidenceRequirement: event.target.value })} placeholder="e.g. corrected paper" /></label></div><div className="form-actions"><button className="button button--primary" disabled={saving}>{saving ? "Prioritizing…" : "Add to plan"}</button></div></form>;
}

function ResourcesView({ snapshot, notify, refresh }: { snapshot: DashboardSnapshot; notify: Notify; refresh: () => Promise<DashboardSnapshot> }) {
  const [status, setStatus] = useState<IndexStatus>({ running: false, paused: false, scanned: 0, indexed: 0, skipped: 0, failed: 0, currentFile: "" });
  const [query, setQuery] = useState("");
  const deferredQuery = useDeferredValue(query);
  const [results, setResults] = useState<ResourceResult[]>([]);
  const [searching, setSearching] = useState(false);
  useEffect(() => {
    const load = () => call<IndexStatus>("get_index_status", {}, status).then(setStatus).catch(() => undefined);
    load(); const timer = window.setInterval(load, 1800); return () => window.clearInterval(timer);
  }, []);
  useEffect(() => {
    setSearching(true); const timer = window.setTimeout(() => call<ResourceResult[]>("search_resources", { query: deferredQuery, limit: 80 }, []).then(setResults).catch((error) => notify("error", readableError(error))).finally(() => setSearching(false)), 250); return () => window.clearTimeout(timer);
  }, [deferredQuery, notify]);
  async function start() { try { await call("start_resource_index", { paths: [] }); setStatus({ ...status, running: true }); notify("success", "Full D-drive IB index started in the background."); } catch (error) { notify("error", readableError(error)); } }
  async function togglePause() { const next = await call<IndexStatus>("set_index_paused", { paused: !status.paused }, { ...status, paused: !status.paused }); setStatus(next); }
  return <div className="resources-layout"><section className="index-banner"><div className="index-icon"><Database size={24} /></div><div className="index-copy"><p className="eyebrow eyebrow--light">Background index</p><h2>{status.running ? status.paused ? "Index paused" : "Reading your IB library" : snapshot.resourceCount ? "Local library ready" : "Build the full local index"}</h2><p>{status.currentFile ? compactPath(status.currentFile) : "Past papers, mark schemes, calculus output and study resources on D."}</p><div className="index-stats"><span><strong>{(snapshot.indexedCount + status.indexed).toLocaleString()}</strong> indexed</span><span><strong>{status.skipped.toLocaleString()}</strong> unchanged</span><span><strong>{status.failed.toLocaleString()}</strong> failed</span></div></div><div className="index-actions">{status.running ? <button className="button button--light" onClick={togglePause}>{status.paused ? <Play size={16} /> : <Pause size={16} />}{status.paused ? "Resume" : "Pause"}</button> : <button className="button button--light" onClick={async () => { await start(); await refresh(); }}><Play size={16} /> Index everything</button>}</div></section><section className="resource-search-panel"><div className="search-box"><Search size={19} /><input aria-label="Search indexed resources" placeholder="Search topic, command term, subject, year or paper…" value={query} onChange={(event) => setQuery(event.target.value)} />{searching && <LoaderCircle className="spin" size={17} />}</div><div className="resource-toolbar"><span>{results.length} results</span><span>Sources remain unchanged</span></div><div className="resource-results">{results.map((result) => <article key={result.id}><div className="file-icon">{result.fileType.toUpperCase().slice(0,4)}</div><div><div className="resource-title"><strong>{result.title}</strong><span className={`state-tag state-tag--${result.extractionState}`}>{result.extractionState.replace("_"," ")}</span></div><p dangerouslySetInnerHTML={{ __html: safeSnippet(result.snippet || compactPath(result.path)) }} /><div className="resource-meta">{result.subjectHint && <span>{result.subjectHint}</span>}{result.yearHint && <span>{result.yearHint}</span>}<span>{formatBytes(result.sizeBytes)}</span></div></div><button className="button button--quiet" onClick={() => call("open_resource", { path: result.path }).catch((error) => notify("error", readableError(error)))}>Open</button></article>)}</div>{!results.length && !searching && <EmptyState icon={FileSearch} title={snapshot.resourceCount ? "No matching resource" : "The index is empty"} body={snapshot.resourceCount ? "Try fewer terms or search by subject and year." : "Start indexing to make the existing D-drive library searchable."} />}</section></div>;
}

function CoreView({ core, notify, refresh }: { core: CoreProgress; notify: Notify; refresh: () => Promise<DashboardSnapshot> }) {
  const [form, setForm] = useState({ ...core }); const [saving, setSaving] = useState(false);
  async function save() { setSaving(true); try { await call("update_core", { input: form }); await refresh(); notify("success", "Core projection updated."); } catch (error) { notify("error", readableError(error)); } finally { setSaving(false); } }
  return <div className="core-page"><section className="core-score-banner"><div><p className="eyebrow eyebrow--light">TOK + EE matrix</p><h2><strong>{core.corePoints}</strong> of 3 core points</h2><p>CAS does not add points, but completion is a diploma requirement.</p></div><div className="core-combination"><span>{form.tokGrade}<small>TOK</small></span><Plus size={18}/><span>{form.eeGrade}<small>EE</small></span><ArrowRight size={18}/><strong>{core.corePoints}</strong></div></section><div className="core-editor-grid"><section className="card core-editor-card"><span className="core-monogram">TOK</span><p className="eyebrow">Theory of knowledge</p><h3>Working grade</h3><div className="grade-picker">{["A","B","C","D","E"].map((grade) => <button className={form.tokGrade === grade ? "is-active" : ""} onClick={() => setForm({ ...form, tokGrade: grade })} key={grade}>{grade}</button>)}</div><label className="field field--stacked"><span>Next milestone</span><textarea rows={3} value={form.tokNextMilestone} onChange={(event) => setForm({ ...form, tokNextMilestone: event.target.value })} /></label></section><section className="card core-editor-card"><span className="core-monogram">EE</span><p className="eyebrow">Extended essay</p><h3>Working grade</h3><div className="grade-picker">{["A","B","C","D","E"].map((grade) => <button className={form.eeGrade === grade ? "is-active" : ""} onClick={() => setForm({ ...form, eeGrade: grade })} key={grade}>{grade}</button>)}</div><label className="field"><span>Current word count</span><input type="number" min={0} max={4000} value={form.eeWordCount} onChange={(event) => setForm({ ...form, eeWordCount: Number(event.target.value) })} /></label><label className="field field--stacked"><span>Next milestone</span><textarea rows={2} value={form.eeNextMilestone} onChange={(event) => setForm({ ...form, eeNextMilestone: event.target.value })} /></label></section><section className={`card core-editor-card cas-editor ${form.casComplete ? "is-complete" : ""}`}><span className="core-monogram">CAS</span><p className="eyebrow">Creativity · activity · service</p><h3>{form.casComplete ? "Requirements met" : "Completion at risk"}</h3><label className="completion-toggle"><input type="checkbox" checked={form.casComplete} onChange={(event) => setForm({ ...form, casComplete: event.target.checked })}/><span className="switch" /><strong>Mark school requirements complete</strong></label><div className="form-grid form-grid--two"><label className="field"><span>Experiences</span><input type="number" min={0} value={form.casExperiences} onChange={(event) => setForm({ ...form, casExperiences: Number(event.target.value) })}/></label><label className="field"><span>Reflections</span><input type="number" min={0} value={form.casReflections} onChange={(event) => setForm({ ...form, casReflections: Number(event.target.value) })}/></label></div><div className="risk-callout"><AlertTriangle size={16}/><span>Use your school's completion decision as the source of truth.</span></div></section></div><div className="sticky-save"><p>Changes recalibrate the dashboard immediately.</p><button className="button button--primary" onClick={save} disabled={saving}>{saving ? "Saving…" : "Save core progress"}</button></div></div>;
}

function CoachView({ snapshot, notify }: { snapshot: DashboardSnapshot; notify: Notify }) {
  const [prompt, setPrompt] = useState(""); const [mode, setMode] = useState("coach"); const [assessedWork, setAssessedWork] = useState(false); const [analysis, setAnalysis] = useState<AiAnalysis>(); const [thinking, setThinking] = useState(false);
  async function submit(event: FormEvent) { event.preventDefault(); if (!prompt.trim()) return; setThinking(true); setAnalysis(undefined); try { setAnalysis(await call("run_ai_analysis", { request: { mode, prompt, assessedWork, context: { profile: snapshot.profile, subjects: snapshot.subjects, core: snapshot.core, projection: snapshot.projection, topTasks: snapshot.tasks.slice(0,8) } } })); } catch (error) { notify("error", readableError(error)); } finally { setThinking(false); } }
  const quickPrompts = ["Build my most realistic seven-day plan", "Where will ten extra hours help most?", "Which subject is least supported by evidence?", "Turn my recurring errors into practice blocks"];
  return <div className="coach-layout"><section className="coach-panel"><div className="coach-intro"><span className="coach-orb"><Sparkles size={24}/></span><div><p className="eyebrow">Evidence-first AI</p><h2>Ask the coach</h2><p>It can reason over your verified profile and selected evidence. It cannot promise a 45 or silently change the plan.</p></div></div><div className="quick-prompts">{quickPrompts.map((value) => <button onClick={() => setPrompt(value)} key={value}>{value}<ArrowRight size={14}/></button>)}</div><form className="coach-form" onSubmit={submit}><textarea rows={7} value={prompt} onChange={(event) => setPrompt(event.target.value)} placeholder="Ask for analysis, a scenario, a plan, or feedback…"/><div className="coach-controls"><div><label><span>Analysis depth</span><select value={mode} onChange={(event) => setMode(event.target.value)}><option value="classify">Quick · Luna</option><option value="coach">Balanced · Terra</option><option value="deep">Deep · Sol</option></select></label><label className="checkbox-label"><input type="checkbox" checked={assessedWork} onChange={(event) => setAssessedWork(event.target.checked)}/><span>Assessed-work drafting</span></label></div><button className="button button--primary" disabled={thinking || !prompt.trim()}>{thinking ? <LoaderCircle className="spin" size={16}/> : <Sparkles size={16}/>} {thinking ? "Analyzing…" : "Analyze"}</button></div></form><div className="privacy-line"><ShieldCheck size={15}/><span>Only this request and the summarized context above are sent. The full library stays local.</span></div></section><section className="coach-output">{thinking && <div className="thinking-state"><div className="thinking-lines"><span/><span/><span/></div><p>Separating evidence from inference…</p></div>}{!thinking && !analysis && <EmptyState icon={BrainCircuit} title="No analysis yet" body="Choose a starting question or describe the decision you need to make."/>}{analysis && <div className="analysis-card"><div className="analysis-header"><div><p className="eyebrow">{analysis.provider} · {analysis.model}</p><h2>{analysis.summary}</h2></div><span className={analysis.provider === "ollama" ? "provider-local" : "provider-cloud"}>{analysis.provider === "ollama" ? <WifiOff size={14}/> : <Cloud size={14}/>} {analysis.provider}</span></div>{analysis.academicIntegrityWarning && <div className="integrity-warning"><AlertTriangle size={18}/><p><strong>Academic integrity warning</strong>{analysis.academicIntegrityWarning}</p></div>}<AnalysisList title="What the evidence supports" items={analysis.claims}/><AnalysisList title="Recommended actions" items={analysis.recommendedActions} numbered/><AnalysisList title="Evidence used" items={analysis.evidence}/><div className="uncertainty-box"><strong>Uncertainty</strong><p>{analysis.uncertainty}</p></div></div>}</section></div>;
}

function AnalysisList({ title, items, numbered = false }: { title: string; items: string[]; numbered?: boolean }) { if (!items.length) return null; return <section className="analysis-section"><h3>{title}</h3><div className="analysis-list">{items.map((item,index) => <div key={`${item}-${index}`}><span>{numbered ? index + 1 : <Check size={13}/>}</span><p>{item}</p></div>)}</div></section>; }

function CalendarView({ notify }: { notify: Notify }) {
  const [calendar, setCalendar] = useState<CalendarStatus>({ connected:false, bindings:[] }); const [loading,setLoading] = useState(true); const [syncing,setSyncing] = useState(false);
  useEffect(() => { call<CalendarStatus>("get_calendar_status", {}, { connected:false, bindings:[] }).then(setCalendar).catch(() => undefined).finally(() => setLoading(false)); }, []);
  async function sync() { setSyncing(true); try { setCalendar(await call("sync_google_calendar")); notify("success","Calendar changes synchronized."); } catch(error){ notify("error",readableError(error)); } finally { setSyncing(false); } }
  async function toggle(binding: CalendarStatus["bindings"][number], field:"selected"|"autoEdit", value:boolean) { try { await call("authorize_calendar", { calendarId:binding.calendarId, selected:field === "selected" ? value : binding.selected, autoEdit:field === "autoEdit" ? value : binding.autoEdit }); setCalendar({ ...calendar, bindings: calendar.bindings.map((item) => item.calendarId === binding.calendarId ? { ...item, [field]: value } : item) }); } catch(error){ notify("error",readableError(error)); } }
  if (loading) return <PageLoader/>;
  return <div className="calendar-layout"><section className="calendar-hero"><div className="calendar-logo"><CalendarDays size={28}/></div><div><p className="eyebrow eyebrow--light">Google Calendar</p><h2>{calendar.connected ? "Your time map is connected" : "Fit the plan around real life"}</h2><p>{calendar.connected ? `${calendar.accountEmail ?? "Connected account"} · ${calendar.bindings.reduce((sum,item) => sum+item.eventCount,0)} events indexed` : "Connect an installed-app OAuth client from Settings. Personal events are read for availability; only authorized calendars can be edited."}</p></div>{calendar.connected && <button className="button button--light" onClick={sync} disabled={syncing}><RefreshCw className={syncing ? "spin" : ""} size={16}/> Sync now</button>}</section>{calendar.connected ? <section className="card calendar-list"><div className="card-header"><div><p className="eyebrow">Authorization boundary</p><h2>Calendar permissions</h2></div><span className="sync-badge"><span className="status-dot"/> Connected</span></div><div className="calendar-table"><div className="calendar-row calendar-row--head"><span>Calendar</span><span>Read for planning</span><span>Auto-edit study blocks</span><span>Events</span></div>{calendar.bindings.map((binding) => <div className="calendar-row" key={binding.calendarId}><span><strong>{binding.name}</strong>{binding.isCoachCalendar && <small>Dedicated coach calendar</small>}</span><label className="mini-toggle"><input type="checkbox" checked={binding.selected} disabled={binding.isCoachCalendar} onChange={(event) => toggle(binding,"selected",event.target.checked)}/><span className="switch"/></label><label className="mini-toggle"><input type="checkbox" checked={binding.autoEdit} disabled={binding.isCoachCalendar || !binding.selected} onChange={(event) => toggle(binding,"autoEdit",event.target.checked)}/><span className="switch"/></label><span>{binding.eventCount}</span></div>)}</div><div className="calendar-safety"><ShieldCheck size={18}/><p><strong>Hard boundary:</strong> attendee-bearing events are never auto-edited, invitations are never sent, and third-party events are never deleted.</p></div></section> : <section className="card connection-empty"><KeyRound size={28}/><h2>Calendar credentials are not configured</h2><p>Create a Google OAuth client of type “Desktop app,” then save the client ID in Settings. The sign-in opens in your normal browser.</p><button className="button button--primary" onClick={() => notify("error","Open Settings & privacy to add the Google client ID first.")}>Connection setup</button></section>}</div>;
}

function DisplayScaleSettings({ value, onChange }: { value: DisplayScale; onChange: (scale: DisplayScale) => void }) {
  return <section className="settings-section display-scale-section"><div className="settings-heading"><div><p className="eyebrow">Accessibility</p><h2>Text & interface size</h2><p>Enlarge the complete app so labels, controls, charts and navigation stay in proportion.</p></div><strong className="scale-readout" aria-live="polite">{Math.round(value * 100)}%</strong></div><fieldset className="scale-options"><legend className="sr-only">Choose interface size</legend>{DISPLAY_SCALE_OPTIONS.map((option) => <label className={value === option.value ? "scale-option scale-option--active" : "scale-option"} key={option.value}><input type="radio" name="display-scale" value={option.value} checked={value === option.value} onChange={() => onChange(option.value)} /><span className="scale-sample" aria-hidden="true" style={{ fontSize: `${12 * option.value}px` }}>Aa</span><span><strong>{option.label}</strong><small>{option.description}</small></span>{value === option.value && <Check size={16} aria-hidden="true" />}</label>)}</fieldset><p className="scale-note">Changes apply immediately and are remembered on this computer.</p></section>;
}

function SettingsView({ notify, onGuidedSetup }: { notify: Notify; onGuidedSetup: () => void }) {
  const [status,setStatus] = useState<SecretStatus>({ openaiConfigured:false,googleConfigured:false,googleConnected:false,ollamaAvailable:false }); const [keys,setKeys] = useState({ openai:"",googleClientId:"",googleClientSecret:"" }); const [saving,setSaving] = useState(""); const [autostart,setAutostart] = useState(false);
  useEffect(() => { call<SecretStatus>("get_secret_status", {}, status).then(setStatus); if (api.native) import("@tauri-apps/plugin-autostart").then(({ isEnabled }) => isEnabled().then(setAutostart)).catch(() => undefined); }, []);
  async function save(name:keyof typeof keys) { if (!keys[name].trim()) return; setSaving(name); try { await call("save_provider_secret", { name, value:keys[name] }); setKeys({ ...keys, [name]:"" }); setStatus(await call("get_secret_status", {}, status)); notify("success","Credential saved in Windows Credential Manager."); } catch(error){ notify("error",readableError(error)); } finally { setSaving(""); } }
  async function connectCalendar() { try { setSaving("calendar"); await call("connect_google_calendar"); setStatus(await call("get_secret_status", {}, status)); notify("success","Google Calendar connected."); } catch(error){ notify("error",readableError(error)); } finally { setSaving(""); } }
  async function toggleAutostart(value:boolean) { try { const plugin=await import("@tauri-apps/plugin-autostart"); value ? await plugin.enable() : await plugin.disable(); setAutostart(value); } catch(error){ notify("error",readableError(error)); } }
  return <div className="settings-layout"><section className="settings-section guided-setup-section"><div className="guided-setup-icon"><MessageCircleQuestion size={24}/></div><div><p className="eyebrow">Easier data entry</p><h2>Guided setup interview</h2><p>Answer one plain-language question at a time. Your current answers are prefilled, and nothing changes until you approve the final review.</p></div><button className="button button--primary" onClick={onGuidedSetup}>Start interview <ArrowRight size={16}/></button></section><section className="settings-section"><div className="settings-heading"><div><p className="eyebrow">Hybrid intelligence</p><h2>AI providers</h2><p>Credentials stay in Windows Credential Manager, not the database or frontend.</p></div></div><div className="provider-card"><div className="provider-icon provider-icon--openai"><BrainCircuit size={20}/></div><div><div className="provider-title"><strong>OpenAI Responses API</strong><StatusLabel active={status.openaiConfigured} activeText="Configured" inactiveText="Optional"/></div><p>Uses Luna for extraction, Terra for regular coaching and Sol only for deep reviews. Requests use <code>store: false</code>.</p><div className="secret-row"><input type="password" placeholder={status.openaiConfigured ? "Replace existing API key" : "sk-…"} value={keys.openai} onChange={(event) => setKeys({ ...keys, openai:event.target.value })}/><button className="button button--quiet" onClick={() => save("openai")} disabled={saving === "openai"}>{saving === "openai" ? "Saving…" : "Save key"}</button></div></div></div><div className="provider-card"><div className="provider-icon"><WifiOff size={20}/></div><div><div className="provider-title"><strong>Ollama · qwen3:4b</strong><StatusLabel active={status.ollamaAvailable} activeText="Available" inactiveText="Not detected"/></div><p>Private offline fallback for lower-confidence coaching when cloud AI is disabled or unavailable.</p></div></div></section><section className="settings-section"><div className="settings-heading"><div><p className="eyebrow">Time integration</p><h2>Google Calendar</h2><p>Use credentials from a Google OAuth “Desktop app” client.</p></div></div><div className="credential-grid"><label className="field"><span>Client ID</span><input type="password" value={keys.googleClientId} onChange={(event) => setKeys({ ...keys, googleClientId:event.target.value })} placeholder={status.googleConfigured ? "Saved · enter to replace" : "…apps.googleusercontent.com"}/><button className="button button--quiet" onClick={() => save("googleClientId")}>Save</button></label><label className="field"><span>Client secret (if supplied)</span><input type="password" value={keys.googleClientSecret} onChange={(event) => setKeys({ ...keys, googleClientSecret:event.target.value })}/><button className="button button--quiet" onClick={() => save("googleClientSecret")}>Save</button></label></div><div className="connection-row"><StatusLabel active={status.googleConnected} activeText="Calendar connected" inactiveText="Not connected"/><button className="button button--primary" disabled={!status.googleConfigured || saving === "calendar"} onClick={connectCalendar}>{saving === "calendar" ? "Waiting for browser…" : status.googleConnected ? "Reconnect" : "Connect Calendar"}</button></div></section><section className="settings-section"><div className="settings-heading"><div><p className="eyebrow">Accountability</p><h2>Windows behavior</h2></div></div><div className="setting-row"><div><strong>Start with Windows</strong><p>Keep reminders and the tray coach available after sign-in.</p></div><label className="mini-toggle"><input type="checkbox" checked={autostart} onChange={(event) => toggleAutostart(event.target.checked)}/><span className="switch"/></label></div><div className="setting-row"><div><strong>Native notifications</strong><p>Send upcoming-block and follow-up reminders through Windows.</p></div><button className="button button--quiet" onClick={() => call("send_test_notification").then(() => notify("success","Test notification sent.")).catch((error) => notify("error",readableError(error)))}>Send test</button></div></section><section className="settings-section"><div className="settings-heading"><div><p className="eyebrow">Recovery</p><h2>Encrypted local backup</h2><p>Create a point-in-time copy under D:\IB45Coach\backups.</p></div><button className="button button--quiet" onClick={() => call<string>("create_backup").then((path) => notify("success",`Backup created: ${path}`)).catch((error) => notify("error",readableError(error)))}><Database size={16}/> Back up now</button></div></section></div>;
}

function StatusLabel({ active, activeText, inactiveText }: { active:boolean;activeText:string;inactiveText:string }) { return <span className={`status-label ${active ? "status-label--active" : ""}`}><i/>{active?activeText:inactiveText}</span>; }
function EmptyState({ icon:Icon,title,body }:{ icon:typeof BookOpen;title:string;body:string }) { return <div className="empty-state"><Icon size={27}/><h3>{title}</h3><p>{body}</p></div>; }
function PageLoader(){ return <div className="page-loader"><LoaderCircle className="spin"/><span>Loading secure local data…</span></div>; }
type Notify=(tone:"success"|"error",message:string)=>void;
async function completeTask(task:Task,notify:Notify){ const evidence=window.prompt(`Evidence for “${task.title}”`,task.evidenceRequirement||"Completed as planned")??""; if(!evidence.trim()) return; try{ await call("complete_task",{taskId:task.id,evidence,outcome:"completed"}); notify("success","Action completed. The queue will adapt."); }catch(error){notify("error",readableError(error));} }
function subjectName(subjects:Subject[],id?:string){ return subjects.find((subject)=>subject.id===id)?.name??"Core / general"; }
function readableError(error:unknown){ if(error instanceof Error) return error.message; if(typeof error==="string") return error; return "Something went wrong. Your local data was not changed."; }
function isOverdue(value:string){ return new Date(value).getTime()<Date.now(); }
function formatDue(value:string){ try{return `${isOverdue(value)?"Overdue · ":""}${formatDistanceToNow(parseISO(value),{addSuffix:true})}`;}catch{return value;} }
function formatShortDate(value:string){ try{return format(parseISO(value),"d MMM yyyy");}catch{return value;} }
function formatBytes(value:number){ if(value<1024)return `${value} B`;if(value<1024**2)return `${(value/1024).toFixed(0)} KB`;return `${(value/1024**2).toFixed(1)} MB`; }
function compactPath(value:string){ const parts=value.split(/[\\/]/);return parts.length>3?`D:\…\${parts.slice(-2).join("\\")}`:value; }
function safeSnippet(value:string){ return value.replace(/&/g,"&amp;").replace(/</g,(match,index,source)=>source.startsWith("<mark>",index)?match:"&lt;").replace(/>/g,(match,index,source)=>source.slice(Math.max(0,index-5),index+1)==="</mark>"?match:"&gt;").replace(/&lt;mark&gt;/g,"<mark>").replace(/&lt;\/mark&gt;/g,"</mark>"); }

export default App;
