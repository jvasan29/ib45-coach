mod ai;
mod calendar;
mod db;
mod exam;
mod models;
mod path_safety;
mod resources;
mod scoring;

use crate::{db::AppStore, models::*, resources::ResourceIndexer};
use chrono::{Duration, Utc};
use rusqlite::params;
use std::{io::Write, path::PathBuf};
use tauri::{
    AppHandle, Manager, State, WindowEvent,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};
use tauri_plugin_notification::NotificationExt;
use uuid::Uuid;

type CommandResult<T> = Result<T, String>;

pub(crate) fn append_startup_log(message: &str) {
    let path = PathBuf::from(r"D:\IB45Coach\logs\startup.log");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{}\t{}", Utc::now().to_rfc3339(), message);
    }
}

fn command_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[tauri::command]
fn initialize_app(store: State<'_, AppStore>) -> CommandResult<DashboardSnapshot> {
    store.dashboard().map_err(command_error)
}

#[tauri::command]
fn complete_onboarding(
    store: State<'_, AppStore>,
    input: OnboardingInput,
) -> CommandResult<DashboardSnapshot> {
    store.save_onboarding(input).map_err(command_error)
}

#[tauri::command]
fn get_subjects(store: State<'_, AppStore>) -> CommandResult<Vec<Subject>> {
    store.subjects().map_err(command_error)
}

#[tauri::command]
fn add_assessment(
    store: State<'_, AppStore>,
    input: AssessmentInput,
) -> CommandResult<AssessmentRecord> {
    store.add_assessment(input).map_err(command_error)
}

#[tauri::command]
fn get_assessments(
    store: State<'_, AppStore>,
    subject_id: String,
) -> CommandResult<Vec<AssessmentRecord>> {
    store.assessments(&subject_id).map_err(command_error)
}

#[tauri::command]
fn update_core(store: State<'_, AppStore>, input: CoreUpdate) -> CommandResult<CoreProgress> {
    store.update_core(input).map_err(command_error)
}

#[tauri::command]
fn create_task(store: State<'_, AppStore>, input: TaskInput) -> CommandResult<Task> {
    store.save_task(input).map_err(command_error)
}

#[tauri::command]
fn get_tasks(store: State<'_, AppStore>, include_completed: bool) -> CommandResult<Vec<Task>> {
    store.tasks(include_completed).map_err(command_error)
}

#[tauri::command]
fn complete_task(
    store: State<'_, AppStore>,
    task_id: String,
    evidence: String,
    outcome: String,
) -> CommandResult<Task> {
    store
        .complete_task(&task_id, &evidence, &outcome)
        .map_err(command_error)
}

#[tauri::command]
fn start_resource_index(
    store: State<'_, AppStore>,
    indexer: State<'_, ResourceIndexer>,
    paths: Vec<String>,
) -> CommandResult<()> {
    indexer
        .start(store.inner().clone(), paths)
        .map_err(command_error)
}

#[tauri::command]
fn set_index_paused(indexer: State<'_, ResourceIndexer>, paused: bool) -> IndexStatus {
    indexer.set_paused(paused);
    indexer.status()
}

#[tauri::command]
fn cancel_resource_index(indexer: State<'_, ResourceIndexer>) -> IndexStatus {
    indexer.cancel();
    indexer.status()
}

#[tauri::command]
fn get_index_status(indexer: State<'_, ResourceIndexer>) -> IndexStatus {
    indexer.status()
}

#[tauri::command]
fn search_resources(
    store: State<'_, AppStore>,
    query: String,
    limit: Option<i64>,
) -> CommandResult<Vec<ResourceResult>> {
    resources::search(&store, &query, limit.unwrap_or(50).clamp(1, 200)).map_err(command_error)
}

#[tauri::command]
fn open_resource(path: String) -> CommandResult<()> {
    let requested = PathBuf::from(&path);
    let canonical = requested.canonicalize().map_err(command_error)?;
    if !path_safety::is_canonical_drive_d(&canonical) {
        return Err("This indexed file is no longer on drive D. Move it back or reindex the library.".into());
    }
    open::that(canonical).map_err(command_error)
}

#[tauri::command]
fn get_exam_library(store: State<'_, AppStore>, subject_id: String, query: String) -> CommandResult<ExamLibrary> {
    exam::library(&store, &subject_id, &query).map_err(command_error)
}

#[tauri::command]
fn create_exam_attempt(store: State<'_, AppStore>, input: ExamAttemptInput) -> CommandResult<ExamAttempt> {
    exam::create_attempt(&store, input).map_err(command_error)
}

#[tauri::command]
fn get_exam_attempt(store: State<'_, AppStore>, attempt_id: String) -> CommandResult<ExamAttempt> {
    exam::get_attempt(&store, &attempt_id).map_err(command_error)
}

#[tauri::command]
fn get_exam_attempts(store: State<'_, AppStore>, subject_id: Option<String>) -> CommandResult<Vec<ExamAttemptSummary>> {
    exam::attempts(&store, subject_id.as_deref()).map_err(command_error)
}

#[tauri::command]
fn save_exam_answer(store: State<'_, AppStore>, input: ExamAnswerInput) -> CommandResult<ExamAnswer> {
    exam::save_answer(&store, input).map_err(command_error)
}

#[tauri::command]
fn submit_exam_attempt(store: State<'_, AppStore>, attempt_id: String) -> CommandResult<ExamAttempt> {
    exam::submit_attempt(&store, &attempt_id).map_err(command_error)
}

#[tauri::command]
fn score_exam_manually(store: State<'_, AppStore>, input: ManualExamScoreInput) -> CommandResult<ExamAttempt> {
    exam::manual_score(&store, input).map_err(command_error)
}

#[tauri::command]
fn get_exam_pdf(store: State<'_, AppStore>, document_id: String) -> CommandResult<ExamPdfPayload> {
    exam::pdf_payload(&store, &document_id).map_err(command_error)
}

#[tauri::command]
async fn run_ai_analysis(
    store: State<'_, AppStore>,
    request: AiRequest,
) -> CommandResult<AiAnalysis> {
    ai::analyze(&store, request).await.map_err(command_error)
}

#[tauri::command]
async fn get_secret_status(store: State<'_, AppStore>) -> CommandResult<SecretStatus> {
    Ok(ai::status(&store).await)
}

#[tauri::command]
fn save_provider_secret(
    store: State<'_, AppStore>,
    name: String,
    value: String,
) -> CommandResult<()> {
    let key = match name.as_str() {
        "openai" => "openai-api-key",
        "googleClientId" => "google-client-id",
        "googleClientSecret" => "google-client-secret",
        _ => return Err("Unknown provider credential".into()),
    };
    store.save_secret(key, value.trim()).map_err(command_error)
}

#[tauri::command]
async fn connect_google_calendar(store: State<'_, AppStore>) -> CommandResult<CalendarStatus> {
    calendar::connect(&store).await.map_err(command_error)
}

#[tauri::command]
async fn sync_google_calendar(store: State<'_, AppStore>) -> CommandResult<CalendarStatus> {
    calendar::sync(&store).await.map_err(command_error)
}

#[tauri::command]
async fn get_calendar_status(store: State<'_, AppStore>) -> CommandResult<CalendarStatus> {
    calendar::status_view(&store).await.map_err(command_error)
}

#[tauri::command]
fn authorize_calendar(
    store: State<'_, AppStore>,
    calendar_id: String,
    selected: bool,
    auto_edit: bool,
) -> CommandResult<()> {
    calendar::set_binding(&store, &calendar_id, selected, auto_edit).map_err(command_error)
}

#[tauri::command]
async fn schedule_study_block(
    store: State<'_, AppStore>,
    input: StudyBlockInput,
) -> CommandResult<String> {
    calendar::schedule_block(&store, input)
        .await
        .map_err(command_error)
}

#[tauri::command]
fn disconnect_google_calendar(store: State<'_, AppStore>) -> CommandResult<()> {
    calendar::disconnect(&store).map_err(command_error)
}

#[tauri::command]
fn send_test_notification(app: AppHandle) -> CommandResult<()> {
    app.notification()
        .builder()
        .title("IB 45 Coach")
        .body("Notifications are ready. Your next study block will appear here.")
        .show()
        .map_err(command_error)
}

#[tauri::command]
fn create_backup(store: State<'_, AppStore>) -> CommandResult<String> {
    store
        .backup()
        .map(|path| path.display().to_string())
        .map_err(command_error)
}

fn start_notification_scheduler(app: AppHandle, store: AppStore) {
    tauri::async_runtime::spawn(async move {
        loop {
            let now = Utc::now();
            let soon = (now + Duration::minutes(15)).to_rfc3339();
            let now_text = now.to_rfc3339();
            if let Ok(connection) = store.connect() {
                let due: Option<(String, String)> = connection.query_row(
                    "SELECT t.id,t.title FROM tasks t
                     WHERE t.status='open' AND t.due_at BETWEEN ?1 AND ?2
                       AND NOT EXISTS(SELECT 1 FROM notification_log n WHERE n.task_id=t.id AND n.notification_type='upcoming' AND n.sent_at > datetime('now','-12 hours'))
                     ORDER BY t.due_at LIMIT 1",
                    params![now_text,soon], |row| Ok((row.get(0)?,row.get(1)?)),
                ).ok();
                if let Some((task_id, title)) = due {
                    if app
                        .notification()
                        .builder()
                        .title("Study block in the next 15 minutes")
                        .body(&title)
                        .show()
                        .is_ok()
                    {
                        let _ = connection.execute(
                            "INSERT INTO notification_log(id,task_id,notification_type,sent_at) VALUES(?1,?2,'upcoming',?3)",
                            params![Uuid::new_v4().to_string(),task_id,Utc::now().to_rfc3339()],
                        );
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    append_startup_log("process start");
    let store = AppStore::initialize()
        .expect("IB 45 Coach could not initialize its encrypted database on D");
    append_startup_log("encrypted store initialized");
    let scheduler_store = store.clone();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--minimized"])
                .build(),
        )
        .manage(store)
        .manage(ResourceIndexer::default())
        .invoke_handler(tauri::generate_handler![
            initialize_app,
            complete_onboarding,
            get_subjects,
            add_assessment,
            get_assessments,
            update_core,
            create_task,
            get_tasks,
            complete_task,
            start_resource_index,
            set_index_paused,
            cancel_resource_index,
            get_index_status,
            search_resources,
            open_resource,
            get_exam_library,
            create_exam_attempt,
            get_exam_attempt,
            get_exam_attempts,
            save_exam_answer,
            submit_exam_attempt,
            score_exam_manually,
            get_exam_pdf,
            run_ai_analysis,
            get_secret_status,
            save_provider_secret,
            connect_google_calendar,
            sync_google_calendar,
            get_calendar_status,
            authorize_calendar,
            schedule_study_block,
            disconnect_google_calendar,
            send_test_notification,
            create_backup,
        ])
        .setup(move |app| {
            append_startup_log("tauri setup entered");
            let show = MenuItem::with_id(app, "show", "Show IB 45 Coach", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            let mut tray = TrayIconBuilder::new()
                .menu(&menu)
                .show_menu_on_left_click(false);
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.on_menu_event(|app, event| match event.id.as_ref() {
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "quit" => app.exit(0),
                _ => {}
            })
            .build(app)?;
            append_startup_log("tray created");
            start_notification_scheduler(app.handle().clone(), scheduler_store.clone());
            append_startup_log("notification scheduler started");
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running IB 45 Coach");
}
