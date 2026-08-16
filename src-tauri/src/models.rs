use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentProfile {
    pub id: String,
    pub name: String,
    pub exam_session: String,
    pub timezone: String,
    pub weekly_capacity_minutes: i64,
    pub sleep_start: String,
    pub sleep_end: String,
    pub school_ai_policy: String,
    pub onboarding_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subject {
    pub id: String,
    pub name: String,
    pub level: String,
    pub group_number: i64,
    pub syllabus_version: String,
    pub current_grade: i64,
    pub target_grade: i64,
    pub confidence: f64,
    pub accent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubjectInput {
    pub name: String,
    pub level: String,
    pub group_number: i64,
    pub syllabus_version: String,
    pub current_grade: i64,
    pub target_grade: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingInput {
    pub name: String,
    pub exam_session: String,
    pub timezone: String,
    pub weekly_capacity_minutes: i64,
    pub sleep_start: String,
    pub sleep_end: String,
    pub school_ai_policy: String,
    pub subjects: Vec<SubjectInput>,
    pub tok_grade: String,
    pub ee_grade: String,
    pub cas_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentInput {
    pub subject_id: String,
    pub title: String,
    pub assessment_type: String,
    pub component: String,
    pub score: f64,
    pub max_score: f64,
    pub weight: f64,
    pub ib_grade: Option<i64>,
    pub occurred_at: String,
    pub feedback: String,
    pub why_lost_marks: String,
    pub error_categories: Vec<String>,
    pub attachment_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentRecord {
    pub id: String,
    pub subject_id: String,
    pub title: String,
    pub assessment_type: String,
    pub component: String,
    pub percentage: f64,
    pub ib_grade: Option<i64>,
    pub occurred_at: String,
    pub feedback: String,
    pub why_lost_marks: String,
    pub error_categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreProgress {
    pub tok_grade: String,
    pub ee_grade: String,
    pub cas_complete: bool,
    pub cas_experiences: i64,
    pub cas_reflections: i64,
    pub ee_word_count: i64,
    pub ee_next_milestone: String,
    pub tok_next_milestone: String,
    pub core_points: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreUpdate {
    pub tok_grade: String,
    pub ee_grade: String,
    pub cas_complete: bool,
    pub cas_experiences: i64,
    pub cas_reflections: i64,
    pub ee_word_count: i64,
    pub ee_next_milestone: String,
    pub tok_next_milestone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub subject_id: Option<String>,
    pub title: String,
    pub rationale: String,
    pub status: String,
    pub due_at: String,
    pub effort_minutes: i64,
    pub expected_impact: f64,
    pub priority_score: f64,
    pub evidence_requirement: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskInput {
    pub subject_id: Option<String>,
    pub title: String,
    pub rationale: String,
    pub due_at: String,
    pub effort_minutes: i64,
    pub expected_impact: f64,
    pub evidence_requirement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Projection {
    pub subject_points: i64,
    pub core_points: i64,
    pub total_points: i64,
    pub low: i64,
    pub high: i64,
    pub target_gap: i64,
    pub confidence: f64,
    pub cas_risk: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSnapshot {
    pub onboarded: bool,
    pub profile: Option<StudentProfile>,
    pub subjects: Vec<Subject>,
    pub core: CoreProgress,
    pub projection: Projection,
    pub tasks: Vec<Task>,
    pub overdue_count: i64,
    pub resource_count: i64,
    pub indexed_count: i64,
    pub next_deadline: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexStatus {
    pub running: bool,
    pub paused: bool,
    pub scanned: u64,
    pub indexed: u64,
    pub skipped: u64,
    pub failed: u64,
    pub current_file: String,
    pub started_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceResult {
    pub id: String,
    pub title: String,
    pub path: String,
    pub file_type: String,
    pub size_bytes: i64,
    pub subject_hint: Option<String>,
    pub year_hint: Option<i64>,
    pub extraction_state: String,
    pub snippet: String,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiRequest {
    pub mode: String,
    pub prompt: String,
    pub context: serde_json::Value,
    pub assessed_work: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiAnalysis {
    pub id: String,
    pub provider: String,
    pub model: String,
    pub mode: String,
    pub summary: String,
    pub claims: Vec<String>,
    pub uncertainty: String,
    pub evidence: Vec<String>,
    pub recommended_actions: Vec<String>,
    pub academic_integrity_warning: Option<String>,
    pub raw: serde_json::Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretStatus {
    pub openai_configured: bool,
    pub google_configured: bool,
    pub google_connected: bool,
    pub ollama_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarBinding {
    pub calendar_id: String,
    pub name: String,
    pub selected: bool,
    pub auto_edit: bool,
    pub is_coach_calendar: bool,
    pub event_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarStatus {
    pub connected: bool,
    pub account_email: Option<String>,
    pub last_sync_at: Option<String>,
    pub bindings: Vec<CalendarBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyBlockInput {
    pub task_id: String,
    pub calendar_id: String,
    pub title: String,
    pub start_at: String,
    pub end_at: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamPaperCandidate {
    pub id: String,
    pub title: String,
    pub path: String,
    pub subject_hint: Option<String>,
    pub year_hint: Option<i64>,
    pub detected_mode: String,
    pub suggested_mark_scheme_id: Option<String>,
    pub suggested_mark_scheme_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamMarkSchemeCandidate {
    pub id: String,
    pub title: String,
    pub path: String,
    pub year_hint: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamLibrary {
    pub papers: Vec<ExamPaperCandidate>,
    pub mark_schemes: Vec<ExamMarkSchemeCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamAttemptInput {
    pub subject_id: String,
    pub paper_document_id: String,
    pub mark_scheme_document_id: Option<String>,
    pub mode: String,
    pub duration_minutes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamAnswerInput {
    pub answer_id: Option<String>,
    pub attempt_id: String,
    pub question_number: Option<i64>,
    pub page_number: Option<i64>,
    pub answer_text: String,
    pub mcq_choice: Option<String>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamAnswer {
    pub id: String,
    pub question_number: Option<i64>,
    pub page_number: Option<i64>,
    pub answer_text: String,
    pub mcq_choice: Option<String>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamAttempt {
    pub id: String,
    pub subject_id: String,
    pub subject_name: String,
    pub paper_document_id: String,
    pub paper_title: String,
    pub mark_scheme_document_id: Option<String>,
    pub mark_scheme_title: Option<String>,
    pub mode: String,
    pub duration_minutes: i64,
    pub status: String,
    pub started_at: String,
    pub ends_at: String,
    pub submitted_at: Option<String>,
    pub score: Option<f64>,
    pub max_score: Option<f64>,
    pub percentage: Option<f64>,
    pub manual_feedback: String,
    pub question_count: i64,
    pub answers: Vec<ExamAnswer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamAttemptSummary {
    pub id: String,
    pub subject_id: String,
    pub subject_name: String,
    pub paper_title: String,
    pub mode: String,
    pub status: String,
    pub started_at: String,
    pub ends_at: String,
    pub submitted_at: Option<String>,
    pub score: Option<f64>,
    pub max_score: Option<f64>,
    pub percentage: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualExamScoreInput {
    pub attempt_id: String,
    pub score: f64,
    pub max_score: f64,
    pub feedback: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamPdfPayload {
    pub document_id: String,
    pub title: String,
    pub data_base64: String,
}
