import { useCallback, useEffect, useRef, useState } from "react";
import {
  AlertTriangle, ArrowLeft, ArrowRight, Check, CheckCircle2, ChevronLeft, ChevronRight,
  Clock3, FileCheck2, FilePenLine, LoaderCircle, Play, Save, Search, ShieldCheck, TimerReset, X,
} from "lucide-react";
import { GlobalWorkerOptions, getDocument, type PDFDocumentProxy } from "pdfjs-dist";
import pdfWorker from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import { call } from "./api";
import { answeredQuestionCount, examQuestionNumbers, formatExamTime, remainingExamSeconds } from "./examLogic";
import type {
  ExamAnswer, ExamAttempt, ExamAttemptSummary, ExamLibrary,
  ExamPaperCandidate, ExamPdfPayload, Subject,
} from "./types";

GlobalWorkerOptions.workerSrc = pdfWorker;

type Notify = (tone: "success" | "error", message: string) => void;
type ExamMode = "mcq" | "theory";

export function ExamLab({ subjects, notify }: { subjects: Subject[]; notify: Notify }) {
  const [subjectId, setSubjectId] = useState(subjects[0]?.id ?? "");
  const [query, setQuery] = useState("");
  const [library, setLibrary] = useState<ExamLibrary>({ papers: [], markSchemes: [] });
  const [history, setHistory] = useState<ExamAttemptSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [paper, setPaper] = useState<ExamPaperCandidate>();
  const [mode, setMode] = useState<ExamMode>("theory");
  const [duration, setDuration] = useState(60);
  const [markSchemeId, setMarkSchemeId] = useState("");
  const [attempt, setAttempt] = useState<ExamAttempt>();
  const [pdf, setPdf] = useState<PDFDocumentProxy>();
  const [opening, setOpening] = useState(false);

  const loadLibrary = useCallback(async () => {
    if (!subjectId) return;
    setLoading(true);
    try {
      const [nextLibrary, nextHistory] = await Promise.all([
        call<ExamLibrary>("get_exam_library", { subjectId, query }, { papers: [], markSchemes: [] }),
        call<ExamAttemptSummary[]>("get_exam_attempts", { subjectId }, []),
      ]);
      setLibrary(nextLibrary);
      setHistory(nextHistory);
    } catch (error) {
      notify("error", readableError(error));
    } finally { setLoading(false); }
  }, [subjectId, query, notify]);

  useEffect(() => { const timer = window.setTimeout(loadLibrary, 250); return () => window.clearTimeout(timer); }, [loadLibrary]);

  function choosePaper(next: ExamPaperCandidate) {
    setPaper(next);
    setMode(next.detectedMode);
    setDuration(next.detectedMode === "mcq" ? 45 : 90);
    setMarkSchemeId(next.suggestedMarkSchemeId ?? "");
  }

  async function openAttempt(next: ExamAttempt) {
    setOpening(true);
    try {
      const payload = await call<ExamPdfPayload>("get_exam_pdf", { documentId: next.paperDocumentId });
      const binary = Uint8Array.from(atob(payload.dataBase64), (character) => character.charCodeAt(0));
      const document = await getDocument({ data: binary }).promise;
      setAttempt(next);
      setPdf(document);
      setPaper(undefined);
    } catch (error) {
      notify("error", readableError(error));
    } finally { setOpening(false); }
  }

  async function startAttempt() {
    if (!paper) return;
    setOpening(true);
    try {
      const next = await call<ExamAttempt>("create_exam_attempt", { input: {
        subjectId, paperDocumentId: paper.id, markSchemeDocumentId: markSchemeId || null, mode, durationMinutes: duration,
      } });
      await openAttempt(next);
    } catch (error) {
      notify("error", readableError(error));
      setOpening(false);
    }
  }

  async function resumeAttempt(summary: ExamAttemptSummary) {
    setOpening(true);
    try { await openAttempt(await call<ExamAttempt>("get_exam_attempt", { attemptId: summary.id })); }
    catch (error) { notify("error", readableError(error)); setOpening(false); }
  }

  async function leaveAttempt() {
    await pdf?.cleanup();
    setPdf(undefined); setAttempt(undefined); await loadLibrary();
  }

  if (attempt && pdf) return <ExamSession attempt={attempt} setAttempt={setAttempt} pdf={pdf} notify={notify} onExit={leaveAttempt}/>;

  const active = history.filter((item) => item.status === "active");
  const completed = history.filter((item) => item.status !== "active").slice(0, 6);
  return <div className="exam-lab">
    <section className="exam-hero"><div className="exam-hero-icon"><FilePenLine size={26}/></div><div><p className="eyebrow eyebrow--light">Timed practice workspace</p><h2>Past Paper Exam Lab</h2><p>Work directly over a preserved PDF copy. MCQs are checked privately against the paired mark scheme; theory papers stay ready for manual marking.</p></div><div className="exam-privacy"><ShieldCheck size={17}/><span>Original files unchanged</span></div></section>
    <nav className="exam-subject-tabs" aria-label="Choose one of your six subjects">{subjects.map((subject) => <button className={subject.id === subjectId ? "is-active" : ""} key={subject.id} onClick={() => { setSubjectId(subject.id); setPaper(undefined); }}><span style={{ background:subject.accent }}/>{subject.name}<small>{subject.level}</small></button>)}</nav>
    {active.length > 0 && <section className="exam-resume"><div><TimerReset size={19}/><span><strong>{active.length} exam{active.length > 1 ? "s" : ""} in progress</strong><small>The timer continues even if the window is closed.</small></span></div>{active.map((item) => <button className="button button--primary" onClick={() => resumeAttempt(item)} key={item.id}>Resume {item.paperTitle} <ArrowRight size={15}/></button>)}</section>}
    <div className="exam-browser-layout"><section className="exam-browser"><div className="exam-browser-toolbar"><div><p className="eyebrow">Indexed on drive D</p><h2>Choose a paper</h2></div><label className="exam-search"><Search size={16}/><span className="sr-only">Search papers</span><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Year, session or paper…"/></label></div>
      {loading ? <div className="exam-loading"><LoaderCircle className="spin" size={22}/> Pairing papers and mark schemes…</div> : library.papers.length === 0 ? <div className="exam-empty"><FileCheck2 size={28}/><h3>No matching indexed PDFs yet</h3><p>Run Resource Library indexing, or adjust the search. Only papers matching this configured subject are shown.</p></div> : <div className="exam-paper-list">{library.papers.slice(0,80).map((item) => <button className={paper?.id === item.id ? "exam-paper is-selected" : "exam-paper"} onClick={() => choosePaper(item)} key={item.id}><span className="exam-paper-type">{item.detectedMode === "mcq" ? "MCQ" : "PDF"}</span><span><strong>{item.title}</strong><small>{item.yearHint ?? "Year unknown"} · {item.suggestedMarkSchemeId ? "Mark scheme paired" : "Choose mark scheme"}</small></span>{item.suggestedMarkSchemeId ? <CheckCircle2 size={17}/> : <ChevronRight size={17}/>}</button>)}</div>}
    </section>
    <aside className="exam-config">{paper ? <><button className="exam-config-close" onClick={() => setPaper(undefined)} aria-label="Close paper setup"><X size={17}/></button><p className="eyebrow">Attempt setup</p><h3>{paper.title}</h3><p className="exam-path" title={paper.path}>{paper.path}</p><fieldset className="exam-mode"><legend>Paper type</legend><button className={mode === "mcq" ? "is-active" : ""} onClick={() => setMode("mcq")}><CheckCircle2 size={17}/><span><strong>MCQ</strong><small>Automatic scoring</small></span></button><button className={mode === "theory" ? "is-active" : ""} onClick={() => setMode("theory")}><FilePenLine size={17}/><span><strong>Theory</strong><small>Type on PDF</small></span></button></fieldset><label className="field"><span>Timer duration</span><select value={duration} onChange={(event) => setDuration(Number(event.target.value))}><option value={30}>30 minutes</option><option value={45}>45 minutes</option><option value={60}>60 minutes</option><option value={75}>75 minutes</option><option value={90}>90 minutes</option><option value={120}>120 minutes</option><option value={150}>150 minutes</option><option value={180}>180 minutes</option></select></label><label className="field"><span>Mark scheme {mode === "mcq" && "(required for auto-score)"}</span><select value={markSchemeId} onChange={(event) => setMarkSchemeId(event.target.value)}><option value="">No mark scheme selected</option>{library.markSchemes.map((scheme) => <option value={scheme.id} key={scheme.id}>{scheme.title}</option>)}</select></label>{mode === "mcq" && !markSchemeId && <div className="exam-warning"><AlertTriangle size={16}/>Without a readable mark scheme, the attempt will wait for manual marking.</div>}<button className="button button--primary button--wide exam-start" onClick={startAttempt} disabled={opening}>{opening ? <LoaderCircle className="spin" size={16}/> : <Play size={16}/>} {opening ? "Opening paper…" : "Start timed attempt"}</button><p className="exam-start-note">Starting creates an encrypted attempt and begins the timer immediately.</p></> : <div className="exam-config-empty"><FilePenLine size={30}/><h3>Select a paper</h3><p>Review its type, timer and paired mark scheme before the clock starts.</p></div>}</aside></div>
    {completed.length > 0 && <section className="exam-history"><div className="section-heading"><div><p className="eyebrow">Recent evidence</p><h2>Attempt history</h2></div></div><div className="exam-history-grid">{completed.map((item) => <article key={item.id}><span className={`exam-status exam-status--${item.status}`}>{item.status === "awaiting_manual" ? "Needs marking" : "Graded"}</span><strong>{item.paperTitle}</strong><small>{item.subjectName} · {item.mode.toUpperCase()}</small>{item.percentage != null ? <b>{Math.round(item.percentage)}%</b> : <b>—</b>}<button className="text-button" onClick={() => resumeAttempt(item)}>Open result <ArrowRight size={13}/></button></article>)}</div></section>}
  </div>;
}

function ExamSession({ attempt, setAttempt, pdf, notify, onExit }: { attempt: ExamAttempt; setAttempt: (attempt: ExamAttempt) => void; pdf: PDFDocumentProxy; notify: Notify; onExit: () => void }) {
  const [remaining, setRemaining] = useState(() => remainingExamSeconds(attempt.endsAt));
  const [page, setPage] = useState(1);
  const [saving, setSaving] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const submittedRef = useRef(false);

  const submit = useCallback(async (automatic = false) => {
    if (submitting || submittedRef.current || attempt.status !== "active") return;
    if (!automatic && !window.confirm("Submit this attempt now? You will not be able to change answers after submission.")) return;
    submittedRef.current = true; setSubmitting(true);
    try {
      const next = await call<ExamAttempt>("submit_exam_attempt", { attemptId:attempt.id });
      setAttempt(next);
      notify("success", next.status === "graded" ? "MCQ scored against the paired mark scheme." : "Paper submitted for manual marking.");
    } catch (error) { submittedRef.current = false; notify("error",readableError(error)); }
    finally { setSubmitting(false); }
  }, [attempt.id,attempt.status,notify,setAttempt,submitting]);

  useEffect(() => {
    if (attempt.status !== "active") return;
    const tick = () => { const next=remainingExamSeconds(attempt.endsAt); setRemaining(next); if (next === 0) void submit(true); };
    tick(); const timer=window.setInterval(tick,1000); return () => window.clearInterval(timer);
  }, [attempt.endsAt,attempt.status,submit]);

  async function saveMcq(questionNumber:number, choice:string) {
    setSaving(true);
    try {
      const answer = await call<ExamAnswer>("save_exam_answer", { input:{ attemptId:attempt.id,questionNumber,pageNumber:null,answerText:"",mcqChoice:choice,x:null,y:null,width:null,height:null,answerId:null } });
      setAttempt({ ...attempt, answers:[...attempt.answers.filter((item) => item.questionNumber !== questionNumber),answer] });
    } catch(error){ notify("error",readableError(error)); } finally { setSaving(false); }
  }

  async function saveTheory(input: Partial<ExamAnswer> & Pick<ExamAnswer,"answerText">) {
    setSaving(true);
    try {
      const answer=await call<ExamAnswer>("save_exam_answer",{input:{attemptId:attempt.id,answerId:input.id ?? null,questionNumber:input.questionNumber ?? null,pageNumber:input.pageNumber ?? page,answerText:input.answerText,mcqChoice:null,x:input.x ?? 8,y:input.y ?? 12,width:input.width ?? 42,height:input.height ?? 16}});
      setAttempt({ ...attempt,answers:[...attempt.answers.filter((item) => item.id !== answer.id),answer] });
      return answer;
    } catch(error){notify("error",readableError(error)); throw error;} finally{setSaving(false);}
  }

  if (attempt.status !== "active") return <ExamResult attempt={attempt} setAttempt={setAttempt} onExit={onExit} notify={notify}/>;
  const answered=answeredQuestionCount(attempt); const questionNumbers=examQuestionNumbers(attempt);
  return <div className="exam-session"><header className="exam-session-bar"><button className="exam-exit" onClick={() => { if (window.confirm("Leave the exam window? Your answers are saved and the timer will continue.")) onExit(); }}><ArrowLeft size={17}/> Exit</button><div className="exam-session-title"><span>{attempt.subjectName}</span><strong>{attempt.paperTitle}</strong></div><div className={remaining < 300 ? "exam-timer exam-timer--urgent" : "exam-timer"}><Clock3 size={18}/><span><small>Time remaining</small><strong aria-live="polite">{formatExamTime(remaining)}</strong></span></div><div className="exam-save-state">{saving ? <LoaderCircle className="spin" size={15}/> : <Save size={15}/>} {saving ? "Saving…" : "Saved"}</div><button className="button button--primary" onClick={() => submit(false)} disabled={submitting}>{submitting ? "Submitting…" : "Submit paper"}</button></header>
    {attempt.mode === "mcq" ? <div className="mcq-workspace"><aside className="mcq-progress"><p className="eyebrow">Answer sheet</p><h2>{answered} / {questionNumbers.length}</h2><div className="mcq-question-map">{questionNumbers.map((number) => <a href={`#mcq-${number}`} className={attempt.answers.some((answer) => answer.questionNumber === number && answer.mcqChoice) ? "is-answered" : ""} key={number}>{number}</a>)}</div><div className="exam-warning"><ShieldCheck size={16}/>Correct answers stay hidden until submission.</div></aside><ReadOnlyPdf pdf={pdf}/><main className="mcq-questions">{questionNumbers.map((number) => { const selected=attempt.answers.find((answer) => answer.questionNumber === number)?.mcqChoice; return <fieldset id={`mcq-${number}`} className="mcq-card" key={number}><legend>Question {number}</legend><div>{["A","B","C","D"].map((choice) => <label className={selected === choice ? "is-selected" : ""} key={choice}><input type="radio" name={`question-${number}`} checked={selected === choice} onChange={() => saveMcq(number,choice)}/><span>{choice}</span></label>)}</div></fieldset>;})}</main></div> : <div className="theory-workspace"><aside className="pdf-tools"><p className="eyebrow">Paper pages</p><strong>Page {page} of {pdf.numPages}</strong><div className="pdf-page-controls"><button onClick={() => setPage((value) => Math.max(1,value-1))} disabled={page===1} aria-label="Previous page"><ChevronLeft size={18}/></button><button onClick={() => setPage((value) => Math.min(pdf.numPages,value+1))} disabled={page===pdf.numPages} aria-label="Next page"><ChevronRight size={18}/></button></div><p>Click <strong>Add answer box</strong>, then click where you want to type. Answers save automatically.</p><div className="page-answer-count"><FilePenLine size={16}/>{attempt.answers.filter((answer) => answer.pageNumber===page).length} answer boxes on this page</div></aside><PdfAnswerPage pdf={pdf} page={page} answers={attempt.answers.filter((answer) => answer.pageNumber===page)} onCreate={saveTheory} onChange={(next) => { setAttempt({...attempt,answers:attempt.answers.map((item)=>item.id===next.id?next:item)}); void saveTheory(next); }}/></div>}
  </div>;
}

function ReadOnlyPdf({ pdf }:{pdf:PDFDocumentProxy}) {
  const [page,setPage]=useState(1);const canvasRef=useRef<HTMLCanvasElement>(null);const [dimensions,setDimensions]=useState({width:620,height:820});
  useEffect(()=>{let cancelled=false;void pdf.getPage(page).then(async(pdfPage)=>{if(cancelled||!canvasRef.current)return;const base=pdfPage.getViewport({scale:1});const scale=620/base.width;const viewport=pdfPage.getViewport({scale});const output=window.devicePixelRatio||1;const canvas=canvasRef.current;const context=canvas.getContext("2d");if(!context)return;canvas.width=Math.floor(viewport.width*output);canvas.height=Math.floor(viewport.height*output);canvas.style.width=`${viewport.width}px`;canvas.style.height=`${viewport.height}px`;setDimensions({width:viewport.width,height:viewport.height});await pdfPage.render({canvas,canvasContext:context,viewport,transform:output!==1?[output,0,0,output,0,0]:undefined}).promise;});return()=>{cancelled=true;};},[pdf,page]);
  return <section className="mcq-pdf-viewer"><div className="mcq-pdf-toolbar"><strong>Question paper</strong><span>Page {page} / {pdf.numPages}</span><div className="pdf-page-controls"><button onClick={()=>setPage((value)=>Math.max(1,value-1))} disabled={page===1} aria-label="Previous page"><ChevronLeft size={18}/></button><button onClick={()=>setPage((value)=>Math.min(pdf.numPages,value+1))} disabled={page===pdf.numPages} aria-label="Next page"><ChevronRight size={18}/></button></div></div><div className="mcq-pdf-scroll"><div className="mcq-pdf-page" style={dimensions}><canvas ref={canvasRef}/></div></div></section>;
}

function PdfAnswerPage({ pdf, page, answers, onCreate, onChange }: { pdf:PDFDocumentProxy;page:number;answers:ExamAnswer[];onCreate:(answer:Partial<ExamAnswer>&Pick<ExamAnswer,"answerText">)=>Promise<ExamAnswer>;onChange:(answer:ExamAnswer)=>void }) {
  const canvasRef=useRef<HTMLCanvasElement>(null); const stageRef=useRef<HTMLDivElement>(null); const [dimensions,setDimensions]=useState({width:760,height:980}); const [placing,setPlacing]=useState(false); const timers=useRef<Record<string,number>>({});
  useEffect(()=>{let cancelled=false; void pdf.getPage(page).then(async (pdfPage)=>{if(cancelled||!canvasRef.current)return; const base=pdfPage.getViewport({scale:1}); const scale=760/base.width; const viewport=pdfPage.getViewport({scale}); const output=window.devicePixelRatio||1; const canvas=canvasRef.current; const context=canvas.getContext("2d"); if(!context)return; canvas.width=Math.floor(viewport.width*output);canvas.height=Math.floor(viewport.height*output);canvas.style.width=`${viewport.width}px`;canvas.style.height=`${viewport.height}px`;setDimensions({width:viewport.width,height:viewport.height});await pdfPage.render({canvas,canvasContext:context,viewport,transform:output!==1?[output,0,0,output,0,0]:undefined}).promise;});return()=>{cancelled=true;};},[pdf,page]);
  useEffect(()=>()=>Object.values(timers.current).forEach(window.clearTimeout),[]);
  async function place(event:React.MouseEvent<HTMLDivElement>){if(!placing||!stageRef.current)return;const rect=stageRef.current.getBoundingClientRect();setPlacing(false);await onCreate({answerText:"",pageNumber:page,x:Math.min(56,Math.max(1,(event.clientX-rect.left)/rect.width*100)),y:Math.min(82,Math.max(1,(event.clientY-rect.top)/rect.height*100)),width:42,height:16});}
  function edit(answer:ExamAnswer,text:string){const next={...answer,answerText:text};window.clearTimeout(timers.current[answer.id]);timers.current[answer.id]=window.setTimeout(()=>onChange(next),550);}
  return <main className="pdf-answer-workspace"><div className="pdf-answer-toolbar"><button className={placing?"button button--primary":"button button--quiet"} onClick={()=>setPlacing((value)=>!value)}><FilePenLine size={16}/>{placing?"Click a location on the page":"Add answer box"}</button><span>{placing?"Placement mode is active":"Your source PDF will not be modified"}</span></div><div className="pdf-scroll"><div ref={stageRef} className={placing?"pdf-page-stage is-placing":"pdf-page-stage"} style={dimensions} onClick={place}><canvas ref={canvasRef}/>{answers.map((answer)=><div className="pdf-answer-box" style={{left:`${answer.x}%`,top:`${answer.y}%`,width:`${answer.width}%`,height:`${answer.height}%`}} key={answer.id} onClick={(event)=>event.stopPropagation()}><label><span>Question</span><input type="number" min="1" value={answer.questionNumber??""} onChange={(event)=>edit({...answer,questionNumber:event.target.value?Number(event.target.value):undefined},answer.answerText)} placeholder="#"/></label><textarea autoFocus={!answer.answerText} value={answer.answerText} onChange={(event)=>edit(answer,event.target.value)} onBlur={(event)=>onChange({...answer,answerText:event.target.value})} placeholder="Type your answer…"/></div>)}</div></div></main>;
}

function ExamResult({ attempt,setAttempt,onExit,notify }:{attempt:ExamAttempt;setAttempt:(attempt:ExamAttempt)=>void;onExit:()=>void;notify:Notify}){
  const [score,setScore]=useState("");const [maxScore,setMaxScore]=useState("");const [feedback,setFeedback]=useState("");const [saving,setSaving]=useState(false);
  async function saveManual(event:React.FormEvent){event.preventDefault();setSaving(true);try{const next=await call<ExamAttempt>("score_exam_manually",{input:{attemptId:attempt.id,score:Number(score),maxScore:Number(maxScore),feedback}});setAttempt(next);notify("success","Manual result saved to exam history.");}catch(error){notify("error",readableError(error));}finally{setSaving(false);}}
  return <main className="exam-result"><section className="exam-result-card"><div className={attempt.status==="graded"?"result-icon result-icon--graded":"result-icon"}>{attempt.status==="graded"?<Check size={30}/>:<FileCheck2 size={30}/>}</div><p className="eyebrow">Attempt complete</p><h1>{attempt.status==="graded"?"Your result is ready":"Ready for manual marking"}</h1><p>{attempt.paperTitle}</p>{attempt.percentage!=null&&<div className="result-score"><strong>{Math.round(attempt.percentage)}%</strong><span>{attempt.score} / {attempt.maxScore} marks</span></div>}{attempt.status==="awaiting_manual"&&<form className="manual-score-form" onSubmit={saveManual}><div><label className="field"><span>Marks awarded</span><input type="number" min="0" step="0.5" required value={score} onChange={(event)=>setScore(event.target.value)}/></label><label className="field"><span>Maximum marks</span><input type="number" min="0.5" step="0.5" required value={maxScore} onChange={(event)=>setMaxScore(event.target.value)}/></label></div><label className="field"><span>Manual feedback</span><textarea rows={3} value={feedback} onChange={(event)=>setFeedback(event.target.value)} placeholder="Main gaps, question numbers, next correction…"/></label><button className="button button--primary" disabled={saving}>{saving?"Saving…":"Save manual result"}</button></form>}<div className="result-actions"><button className="button button--quiet" onClick={onExit}><ArrowLeft size={16}/> Back to Exam Lab</button></div></section></main>;
}

function readableError(error: unknown) { return error instanceof Error ? error.message : String(error); }
