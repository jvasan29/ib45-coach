use crate::{models::*, scoring};
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use keyring::Entry;
use rand::RngCore;
use rusqlite::{Connection, OptionalExtension, params};
use std::{fs, path::PathBuf, time::Duration};
use uuid::Uuid;

const KEYRING_SERVICE: &str = "IB45Coach";

#[derive(Clone)]
pub struct AppStore {
    pub root: PathBuf,
    pub db_path: PathBuf,
    db_key: String,
}

impl AppStore {
    pub fn initialize() -> Result<Self> {
        let root = std::env::var("IB45_DATA_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(r"D:\IB45Coach"));
        let data_dir = root.join("data");
        for directory in [
            data_dir.clone(),
            root.join("index"),
            root.join("backups"),
            root.join("logs"),
            root.join("models"),
        ] {
            fs::create_dir_all(&directory)
                .with_context(|| format!("Could not create {}", directory.display()))?;
        }
        let db_key = Self::load_or_create_database_key(&data_dir)?;
        let store = Self {
            root,
            db_path: data_dir.join("coach.db"),
            db_key,
        };
        store.migrate()?;
        Ok(store)
    }

    fn load_or_create_database_key(data_dir: &std::path::Path) -> Result<String> {
        if let Ok(entry) = Entry::new(KEYRING_SERVICE, "database-key") {
            if let Ok(value) = entry.get_password() {
                if value.len() >= 32 {
                    return Ok(value);
                }
            }
            let mut bytes = [0u8; 32];
            rand::rng().fill_bytes(&mut bytes);
            let value = hex::encode(bytes);
            if entry.set_password(&value).is_ok() {
                return Ok(value);
            }
        }

        // Credential Manager can be unavailable in stripped-down Windows sessions.
        // This fallback remains on D and is never exposed to the frontend.
        let fallback = data_dir.join(".database-key");
        if fallback.exists() {
            return fs::read_to_string(&fallback).context("Could not read database key");
        }
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        let value = hex::encode(bytes);
        fs::write(&fallback, &value).context("Could not persist database key")?;
        Ok(value)
    }

    pub fn save_secret(&self, name: &str, value: &str) -> Result<()> {
        let entry = Entry::new(KEYRING_SERVICE, name)?;
        entry.set_password(value)?;
        Ok(())
    }

    pub fn get_secret(&self, name: &str) -> Option<String> {
        Entry::new(KEYRING_SERVICE, name)
            .ok()
            .and_then(|entry| entry.get_password().ok())
            .filter(|value| !value.trim().is_empty())
    }

    pub fn delete_secret(&self, name: &str) {
        if let Ok(entry) = Entry::new(KEYRING_SERVICE, name) {
            let _ = entry.delete_credential();
        }
    }

    pub fn connect(&self) -> Result<Connection> {
        let connection = Connection::open(&self.db_path)
            .with_context(|| format!("Could not open {}", self.db_path.display()))?;
        connection.execute_batch(&format!(
            "PRAGMA key = \"x'{}'\";\
             PRAGMA cipher_compatibility = 4;\
             PRAGMA foreign_keys = ON;\
             PRAGMA secure_delete = ON;\
             PRAGMA synchronous = NORMAL;\
             PRAGMA wal_autocheckpoint = 1000;",
            self.db_key
        ))?;
        connection.busy_timeout(Duration::from_secs(30))?;
        connection.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(()))?;
        Ok(connection)
    }

    fn migrate(&self) -> Result<()> {
        let connection = self.connect()?;
        // WAL is persistent for the database and lets the resource indexer write
        // without blocking dashboard and exam reads. Set it once at startup rather
        // than on every connection, where the mode change itself can need a lock.
        connection.execute_batch("PRAGMA journal_mode = WAL;")?;
        let schema = r#"
            CREATE TABLE IF NOT EXISTS app_settings (
              key TEXT PRIMARY KEY,
              value TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS student_profile (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              exam_session TEXT NOT NULL,
              timezone TEXT NOT NULL,
              weekly_capacity_minutes INTEGER NOT NULL,
              sleep_start TEXT NOT NULL,
              sleep_end TEXT NOT NULL,
              school_ai_policy TEXT NOT NULL,
              onboarding_complete INTEGER NOT NULL DEFAULT 0,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS subjects (
              id TEXT PRIMARY KEY,
              profile_id TEXT NOT NULL REFERENCES student_profile(id) ON DELETE CASCADE,
              name TEXT NOT NULL,
              level TEXT NOT NULL CHECK(level IN ('HL','SL')),
              group_number INTEGER NOT NULL,
              syllabus_version TEXT NOT NULL,
              current_grade INTEGER NOT NULL CHECK(current_grade BETWEEN 1 AND 7),
              target_grade INTEGER NOT NULL CHECK(target_grade BETWEEN 1 AND 7),
              confidence REAL NOT NULL DEFAULT 0.35,
              accent TEXT NOT NULL,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS assessment_components (
              id TEXT PRIMARY KEY,
              subject_id TEXT NOT NULL REFERENCES subjects(id) ON DELETE CASCADE,
              name TEXT NOT NULL,
              component_type TEXT NOT NULL,
              weight REAL NOT NULL,
              source TEXT NOT NULL DEFAULT 'school',
              UNIQUE(subject_id, name)
            );

            CREATE TABLE IF NOT EXISTS assessments (
              id TEXT PRIMARY KEY,
              subject_id TEXT NOT NULL REFERENCES subjects(id) ON DELETE CASCADE,
              title TEXT NOT NULL,
              assessment_type TEXT NOT NULL,
              component TEXT NOT NULL,
              score REAL NOT NULL,
              max_score REAL NOT NULL,
              percentage REAL NOT NULL,
              weight REAL NOT NULL,
              ib_grade INTEGER,
              occurred_at TEXT NOT NULL,
              feedback TEXT NOT NULL,
              why_lost_marks TEXT NOT NULL,
              error_categories TEXT NOT NULL,
              attachment_path TEXT,
              created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS core_progress (
              profile_id TEXT PRIMARY KEY REFERENCES student_profile(id) ON DELETE CASCADE,
              tok_grade TEXT NOT NULL DEFAULT 'C',
              ee_grade TEXT NOT NULL DEFAULT 'C',
              cas_complete INTEGER NOT NULL DEFAULT 0,
              cas_experiences INTEGER NOT NULL DEFAULT 0,
              cas_reflections INTEGER NOT NULL DEFAULT 0,
              ee_word_count INTEGER NOT NULL DEFAULT 0,
              ee_next_milestone TEXT NOT NULL DEFAULT '',
              tok_next_milestone TEXT NOT NULL DEFAULT '',
              updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS tasks (
              id TEXT PRIMARY KEY,
              subject_id TEXT REFERENCES subjects(id) ON DELETE SET NULL,
              title TEXT NOT NULL,
              rationale TEXT NOT NULL,
              status TEXT NOT NULL DEFAULT 'open',
              due_at TEXT NOT NULL,
              effort_minutes INTEGER NOT NULL,
              expected_impact REAL NOT NULL,
              priority_score REAL NOT NULL,
              evidence_requirement TEXT NOT NULL,
              completed_at TEXT,
              calendar_event_id TEXT,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS check_ins (
              id TEXT PRIMARY KEY,
              task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
              outcome TEXT NOT NULL,
              evidence TEXT NOT NULL,
              energy INTEGER,
              note TEXT NOT NULL,
              created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS resource_documents (
              id TEXT PRIMARY KEY,
              path TEXT NOT NULL UNIQUE,
              title TEXT NOT NULL,
              file_type TEXT NOT NULL,
              size_bytes INTEGER NOT NULL,
              modified_at TEXT NOT NULL,
              sha256 TEXT,
              duplicate_of TEXT,
              subject_hint TEXT,
              year_hint INTEGER,
              extraction_state TEXT NOT NULL,
              extraction_error TEXT,
              indexed_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_resource_hash ON resource_documents(sha256);
            CREATE INDEX IF NOT EXISTS idx_resource_subject ON resource_documents(subject_hint);

            CREATE TABLE IF NOT EXISTS resource_chunks (
              id TEXT PRIMARY KEY,
              document_id TEXT NOT NULL REFERENCES resource_documents(id) ON DELETE CASCADE,
              chunk_index INTEGER NOT NULL,
              body TEXT NOT NULL,
              embedding_json TEXT,
              UNIQUE(document_id, chunk_index)
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS resource_fts USING fts5(
              chunk_id UNINDEXED,
              document_id UNINDEXED,
              title,
              body,
              path UNINDEXED,
              tokenize='unicode61 remove_diacritics 2'
            );

            CREATE TABLE IF NOT EXISTS ai_analyses (
              id TEXT PRIMARY KEY,
              provider TEXT NOT NULL,
              model TEXT NOT NULL,
              mode TEXT NOT NULL,
              prompt TEXT NOT NULL,
              response_json TEXT NOT NULL,
              created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS calendar_bindings (
              calendar_id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              selected INTEGER NOT NULL DEFAULT 0,
              auto_edit INTEGER NOT NULL DEFAULT 0,
              is_coach_calendar INTEGER NOT NULL DEFAULT 0,
              sync_token TEXT,
              event_count INTEGER NOT NULL DEFAULT 0,
              updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS calendar_events (
              event_id TEXT NOT NULL,
              calendar_id TEXT NOT NULL REFERENCES calendar_bindings(calendar_id) ON DELETE CASCADE,
              summary TEXT NOT NULL,
              start_at TEXT,
              end_at TEXT,
              has_attendees INTEGER NOT NULL DEFAULT 0,
              etag TEXT,
              status TEXT NOT NULL,
              raw_json TEXT NOT NULL,
              updated_at TEXT NOT NULL,
              PRIMARY KEY(event_id, calendar_id)
            );

            CREATE TABLE IF NOT EXISTS audit_events (
              id TEXT PRIMARY KEY,
              action TEXT NOT NULL,
              entity_type TEXT NOT NULL,
              entity_id TEXT,
              before_json TEXT,
              after_json TEXT,
              reversible INTEGER NOT NULL DEFAULT 0,
              created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS notification_log (
              id TEXT PRIMARY KEY,
              task_id TEXT,
              notification_type TEXT NOT NULL,
              sent_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS exam_attempts (
              id TEXT PRIMARY KEY,
              subject_id TEXT NOT NULL REFERENCES subjects(id) ON DELETE CASCADE,
              paper_document_id TEXT NOT NULL REFERENCES resource_documents(id),
              mark_scheme_document_id TEXT REFERENCES resource_documents(id),
              mode TEXT NOT NULL CHECK(mode IN ('mcq','theory')),
              duration_minutes INTEGER NOT NULL,
              status TEXT NOT NULL DEFAULT 'active',
              started_at TEXT NOT NULL,
              ends_at TEXT NOT NULL,
              submitted_at TEXT,
              score REAL,
              max_score REAL,
              percentage REAL,
              answer_key_json TEXT NOT NULL DEFAULT '[]',
              manual_feedback TEXT NOT NULL DEFAULT '',
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS exam_answers (
              id TEXT PRIMARY KEY,
              attempt_id TEXT NOT NULL REFERENCES exam_attempts(id) ON DELETE CASCADE,
              question_number INTEGER,
              page_number INTEGER,
              answer_text TEXT NOT NULL DEFAULT '',
              mcq_choice TEXT,
              x REAL,
              y REAL,
              width REAL,
              height REAL,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            CREATE UNIQUE INDEX IF NOT EXISTS idx_exam_mcq_answer
              ON exam_answers(attempt_id, question_number)
              WHERE question_number IS NOT NULL AND mcq_choice IS NOT NULL;
            CREATE INDEX IF NOT EXISTS idx_exam_attempt_subject ON exam_attempts(subject_id, started_at DESC);
            "#;
        for statement in schema.split(';') {
            let statement = statement.trim();
            if statement.is_empty() {
                continue;
            }
            connection.execute_batch(&format!("{statement};"))?;
        }
        Ok(())
    }

    pub fn dashboard(&self) -> Result<DashboardSnapshot> {
        let connection = self.connect()?;
        let profile = connection
            .query_row(
                "SELECT id,name,exam_session,timezone,weekly_capacity_minutes,sleep_start,sleep_end,school_ai_policy,onboarding_complete FROM student_profile WHERE id='primary'",
                [],
                |row| {
                    Ok(StudentProfile {
                        id: row.get(0)?, name: row.get(1)?, exam_session: row.get(2)?,
                        timezone: row.get(3)?, weekly_capacity_minutes: row.get(4)?,
                        sleep_start: row.get(5)?, sleep_end: row.get(6)?, school_ai_policy: row.get(7)?,
                        onboarding_complete: row.get::<_, i64>(8)? == 1,
                    })
                },
            )
            .optional()?;
        let subjects = Self::read_subjects(&connection)?;
        let core = Self::read_core(&connection)?;
        let tasks = Self::read_tasks(&connection, false)?;
        let subject_points: i64 = subjects.iter().map(|subject| subject.current_grade).sum();
        let confidence = if subjects.is_empty() {
            0.0
        } else {
            subjects
                .iter()
                .map(|subject| subject.confidence)
                .sum::<f64>()
                / subjects.len() as f64
        };
        let core_points = core.core_points;
        let total_points = subject_points + core_points;
        let spread = ((1.0 - confidence) * 5.0).ceil() as i64;
        let now = Utc::now().to_rfc3339();
        let overdue_count = connection.query_row(
            "SELECT count(*) FROM tasks WHERE status='open' AND due_at < ?1",
            [now],
            |row| row.get(0),
        )?;
        let (resource_count, indexed_count) = connection.query_row(
            "SELECT count(*), coalesce(sum(CASE WHEN extraction_state IN ('ready','metadata','duplicate') THEN 1 ELSE 0 END),0) FROM resource_documents",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let next_deadline = connection
            .query_row(
                "SELECT due_at FROM tasks WHERE status='open' ORDER BY due_at LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        Ok(DashboardSnapshot {
            onboarded: profile
                .as_ref()
                .is_some_and(|value| value.onboarding_complete),
            profile,
            subjects,
            core: core.clone(),
            projection: Projection {
                subject_points,
                core_points,
                total_points,
                low: (total_points - spread).max(0),
                high: (total_points + spread).min(45),
                target_gap: (45 - total_points).max(0),
                confidence,
                cas_risk: !core.cas_complete,
            },
            tasks,
            overdue_count,
            resource_count,
            indexed_count,
            next_deadline,
        })
    }

    pub fn save_onboarding(&self, input: OnboardingInput) -> Result<DashboardSnapshot> {
        if input.subjects.len() != 6 {
            return Err(anyhow!(
                "The IB Diploma profile must contain exactly six subjects"
            ));
        }
        let hl_count = input
            .subjects
            .iter()
            .filter(|subject| subject.level == "HL")
            .count();
        if !(3..=4).contains(&hl_count) {
            return Err(anyhow!("Choose three or four Higher Level subjects"));
        }
        let now = Utc::now().to_rfc3339();
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO student_profile(id,name,exam_session,timezone,weekly_capacity_minutes,sleep_start,sleep_end,school_ai_policy,onboarding_complete,created_at,updated_at)
             VALUES('primary',?1,?2,?3,?4,?5,?6,?7,1,?8,?8)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name,exam_session=excluded.exam_session,timezone=excluded.timezone,weekly_capacity_minutes=excluded.weekly_capacity_minutes,sleep_start=excluded.sleep_start,sleep_end=excluded.sleep_end,school_ai_policy=excluded.school_ai_policy,onboarding_complete=1,updated_at=excluded.updated_at",
            params![input.name, input.exam_session, input.timezone, input.weekly_capacity_minutes, input.sleep_start, input.sleep_end, input.school_ai_policy, now],
        )?;
        transaction.execute("DELETE FROM subjects WHERE profile_id='primary'", [])?;
        let accents = [
            "#2f6fed", "#7a5af8", "#0f9f75", "#d97706", "#e5484d", "#1387a3",
        ];
        for (index, subject) in input.subjects.iter().enumerate() {
            transaction.execute(
                "INSERT INTO subjects(id,profile_id,name,level,group_number,syllabus_version,current_grade,target_grade,confidence,accent,created_at,updated_at)
                 VALUES(?1,'primary',?2,?3,?4,?5,?6,?7,0.35,?8,?9,?9)",
                params![Uuid::new_v4().to_string(), subject.name, subject.level, subject.group_number, subject.syllabus_version, subject.current_grade.clamp(1,7), subject.target_grade.clamp(1,7), accents[index], now],
            )?;
        }
        transaction.execute(
            "INSERT INTO core_progress(profile_id,tok_grade,ee_grade,cas_complete,updated_at) VALUES('primary',?1,?2,?3,?4)
             ON CONFLICT(profile_id) DO UPDATE SET tok_grade=excluded.tok_grade,ee_grade=excluded.ee_grade,cas_complete=excluded.cas_complete,updated_at=excluded.updated_at",
            params![input.tok_grade, input.ee_grade, input.cas_complete as i64, now],
        )?;
        transaction.execute(
            "INSERT INTO audit_events(id,action,entity_type,entity_id,after_json,reversible,created_at) VALUES(?1,'complete_onboarding','profile','primary',?2,1,?3)",
            params![Uuid::new_v4().to_string(), serde_json::to_string(&input)?, now],
        )?;
        transaction.commit()?;
        self.dashboard()
    }

    pub fn subjects(&self) -> Result<Vec<Subject>> {
        Self::read_subjects(&self.connect()?)
    }

    fn read_subjects(connection: &Connection) -> Result<Vec<Subject>> {
        let mut statement = connection.prepare(
            "SELECT id,name,level,group_number,syllabus_version,current_grade,target_grade,confidence,accent FROM subjects ORDER BY group_number,name",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(Subject {
                id: row.get(0)?,
                name: row.get(1)?,
                level: row.get(2)?,
                group_number: row.get(3)?,
                syllabus_version: row.get(4)?,
                current_grade: row.get(5)?,
                target_grade: row.get(6)?,
                confidence: row.get(7)?,
                accent: row.get(8)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    fn read_core(connection: &Connection) -> Result<CoreProgress> {
        let value = connection
            .query_row(
                "SELECT tok_grade,ee_grade,cas_complete,cas_experiences,cas_reflections,ee_word_count,ee_next_milestone,tok_next_milestone FROM core_progress WHERE profile_id='primary'",
                [],
                |row| {
                    let tok_grade: String = row.get(0)?;
                    let ee_grade: String = row.get(1)?;
                    Ok(CoreProgress {
                        core_points: scoring::core_points(&tok_grade, &ee_grade),
                        tok_grade, ee_grade, cas_complete: row.get::<_, i64>(2)? == 1,
                        cas_experiences: row.get(3)?, cas_reflections: row.get(4)?, ee_word_count: row.get(5)?,
                        ee_next_milestone: row.get(6)?, tok_next_milestone: row.get(7)?,
                    })
                },
            )
            .optional()?;
        Ok(value.unwrap_or(CoreProgress {
            tok_grade: "C".into(),
            ee_grade: "C".into(),
            cas_complete: false,
            cas_experiences: 0,
            cas_reflections: 0,
            ee_word_count: 0,
            ee_next_milestone: String::new(),
            tok_next_milestone: String::new(),
            core_points: 1,
        }))
    }

    pub fn update_core(&self, input: CoreUpdate) -> Result<CoreProgress> {
        let connection = self.connect()?;
        let before = Self::read_core(&connection)?;
        let now = Utc::now().to_rfc3339();
        connection.execute(
            "INSERT INTO core_progress(profile_id,tok_grade,ee_grade,cas_complete,cas_experiences,cas_reflections,ee_word_count,ee_next_milestone,tok_next_milestone,updated_at)
             VALUES('primary',?1,?2,?3,?4,?5,?6,?7,?8,?9)
             ON CONFLICT(profile_id) DO UPDATE SET tok_grade=excluded.tok_grade,ee_grade=excluded.ee_grade,cas_complete=excluded.cas_complete,cas_experiences=excluded.cas_experiences,cas_reflections=excluded.cas_reflections,ee_word_count=excluded.ee_word_count,ee_next_milestone=excluded.ee_next_milestone,tok_next_milestone=excluded.tok_next_milestone,updated_at=excluded.updated_at",
            params![input.tok_grade, input.ee_grade, input.cas_complete as i64, input.cas_experiences, input.cas_reflections, input.ee_word_count, input.ee_next_milestone, input.tok_next_milestone, now],
        )?;
        let after = Self::read_core(&connection)?;
        self.audit(
            &connection,
            "update_core",
            "core",
            Some("primary"),
            Some(&before),
            Some(&after),
            true,
        )?;
        Ok(after)
    }

    pub fn add_assessment(&self, input: AssessmentInput) -> Result<AssessmentRecord> {
        if input.max_score <= 0.0 || input.score < 0.0 || input.score > input.max_score {
            return Err(anyhow!(
                "Assessment scores must be between zero and the maximum score"
            ));
        }
        let percentage = input.score / input.max_score * 100.0;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let error_json = serde_json::to_string(&input.error_categories)?;
        let connection = self.connect()?;
        connection.execute(
            "INSERT INTO assessments(id,subject_id,title,assessment_type,component,score,max_score,percentage,weight,ib_grade,occurred_at,feedback,why_lost_marks,error_categories,attachment_path,created_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
            params![id,input.subject_id,input.title,input.assessment_type,input.component,input.score,input.max_score,percentage,input.weight,input.ib_grade,input.occurred_at,input.feedback,input.why_lost_marks,error_json,input.attachment_path,now],
        )?;
        let measured_grade = input
            .ib_grade
            .unwrap_or_else(|| scoring::percent_to_provisional_grade(percentage));
        connection.execute(
            "UPDATE subjects SET current_grade=round(current_grade*0.55 + ?1*0.45), confidence=min(0.92,confidence+0.08),updated_at=?2 WHERE id=?3",
            params![measured_grade, now, input.subject_id],
        )?;
        let record = AssessmentRecord {
            id: id.clone(),
            subject_id: input.subject_id,
            title: input.title,
            assessment_type: input.assessment_type,
            component: input.component,
            percentage,
            ib_grade: input.ib_grade,
            occurred_at: input.occurred_at,
            feedback: input.feedback,
            why_lost_marks: input.why_lost_marks,
            error_categories: input.error_categories,
        };
        self.audit(
            &connection,
            "add_assessment",
            "assessment",
            Some(&id),
            None::<&AssessmentRecord>,
            Some(&record),
            true,
        )?;
        Ok(record)
    }

    pub fn assessments(&self, subject_id: &str) -> Result<Vec<AssessmentRecord>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT id,subject_id,title,assessment_type,component,percentage,ib_grade,occurred_at,feedback,why_lost_marks,error_categories FROM assessments WHERE subject_id=?1 ORDER BY occurred_at DESC",
        )?;
        let rows = statement.query_map([subject_id], |row| {
            let categories: String = row.get(10)?;
            Ok(AssessmentRecord {
                id: row.get(0)?,
                subject_id: row.get(1)?,
                title: row.get(2)?,
                assessment_type: row.get(3)?,
                component: row.get(4)?,
                percentage: row.get(5)?,
                ib_grade: row.get(6)?,
                occurred_at: row.get(7)?,
                feedback: row.get(8)?,
                why_lost_marks: row.get(9)?,
                error_categories: serde_json::from_str(&categories).unwrap_or_default(),
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn save_task(&self, input: TaskInput) -> Result<Task> {
        let connection = self.connect()?;
        let weakness_count = input.subject_id.as_ref().map_or(0, |subject_id| {
            connection.query_row(
                "SELECT count(*) FROM assessments WHERE subject_id=?1 AND length(error_categories)>2",
                [subject_id], |row| row.get(0),
            ).unwrap_or(0)
        });
        let task = Task {
            id: Uuid::new_v4().to_string(),
            subject_id: input.subject_id,
            title: input.title,
            rationale: input.rationale,
            status: "open".into(),
            due_at: input.due_at,
            effort_minutes: input.effort_minutes.max(15),
            expected_impact: input.expected_impact,
            priority_score: 0.0,
            evidence_requirement: input.evidence_requirement,
            completed_at: None,
        };
        let mut task = Task {
            priority_score: scoring::task_priority(
                &task.due_at,
                task.effort_minutes,
                task.expected_impact,
                weakness_count,
            ),
            ..task
        };
        let now = Utc::now().to_rfc3339();
        connection.execute(
            "INSERT INTO tasks(id,subject_id,title,rationale,status,due_at,effort_minutes,expected_impact,priority_score,evidence_requirement,created_at,updated_at) VALUES(?1,?2,?3,?4,'open',?5,?6,?7,?8,?9,?10,?10)",
            params![task.id,task.subject_id,task.title,task.rationale,task.due_at,task.effort_minutes,task.expected_impact,task.priority_score,task.evidence_requirement,now],
        )?;
        self.audit(
            &connection,
            "create_task",
            "task",
            Some(&task.id),
            None::<&Task>,
            Some(&task),
            true,
        )?;
        // Ensure no accidental NaN can enter UI serialization.
        if !task.priority_score.is_finite() {
            task.priority_score = 0.0;
        }
        Ok(task)
    }

    pub fn tasks(&self, include_completed: bool) -> Result<Vec<Task>> {
        Self::read_tasks(&self.connect()?, include_completed)
    }

    fn read_tasks(connection: &Connection, include_completed: bool) -> Result<Vec<Task>> {
        let sql = if include_completed {
            "SELECT id,subject_id,title,rationale,status,due_at,effort_minutes,expected_impact,priority_score,evidence_requirement,completed_at FROM tasks ORDER BY CASE WHEN status='open' THEN 0 ELSE 1 END,priority_score DESC,due_at"
        } else {
            "SELECT id,subject_id,title,rationale,status,due_at,effort_minutes,expected_impact,priority_score,evidence_requirement,completed_at FROM tasks WHERE status='open' ORDER BY priority_score DESC,due_at LIMIT 20"
        };
        let mut statement = connection.prepare(sql)?;
        let rows = statement.query_map([], |row| {
            Ok(Task {
                id: row.get(0)?,
                subject_id: row.get(1)?,
                title: row.get(2)?,
                rationale: row.get(3)?,
                status: row.get(4)?,
                due_at: row.get(5)?,
                effort_minutes: row.get(6)?,
                expected_impact: row.get(7)?,
                priority_score: row.get(8)?,
                evidence_requirement: row.get(9)?,
                completed_at: row.get(10)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn complete_task(&self, task_id: &str, evidence: &str, outcome: &str) -> Result<Task> {
        let connection = self.connect()?;
        let before = Self::read_task(&connection, task_id)?;
        let now = Utc::now().to_rfc3339();
        connection.execute(
            "UPDATE tasks SET status='completed',completed_at=?1,updated_at=?1 WHERE id=?2",
            params![now, task_id],
        )?;
        connection.execute(
            "INSERT INTO check_ins(id,task_id,outcome,evidence,note,created_at) VALUES(?1,?2,?3,?4,'',?5)",
            params![Uuid::new_v4().to_string(),task_id,outcome,evidence,now],
        )?;
        let after = Self::read_task(&connection, task_id)?;
        self.audit(
            &connection,
            "complete_task",
            "task",
            Some(task_id),
            Some(&before),
            Some(&after),
            true,
        )?;
        Ok(after)
    }

    fn read_task(connection: &Connection, task_id: &str) -> Result<Task> {
        connection.query_row(
            "SELECT id,subject_id,title,rationale,status,due_at,effort_minutes,expected_impact,priority_score,evidence_requirement,completed_at FROM tasks WHERE id=?1",
            [task_id], |row| Ok(Task {
                id: row.get(0)?, subject_id: row.get(1)?, title: row.get(2)?, rationale: row.get(3)?,
                status: row.get(4)?, due_at: row.get(5)?, effort_minutes: row.get(6)?, expected_impact: row.get(7)?,
                priority_score: row.get(8)?, evidence_requirement: row.get(9)?, completed_at: row.get(10)?,
            }),
        ).map_err(Into::into)
    }

    pub fn save_ai_analysis(&self, analysis: &AiAnalysis, prompt: &str) -> Result<()> {
        let connection = self.connect()?;
        connection.execute(
            "INSERT INTO ai_analyses(id,provider,model,mode,prompt,response_json,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![analysis.id,analysis.provider,analysis.model,analysis.mode,prompt,serde_json::to_string(analysis)?,analysis.created_at],
        )?;
        Ok(())
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.connect()?.execute(
            "INSERT INTO app_settings(key,value,updated_at) VALUES(?1,?2,?3) ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at=excluded.updated_at",
            params![key,value,Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Option<String> {
        self.connect().ok().and_then(|connection| {
            connection
                .query_row(
                    "SELECT value FROM app_settings WHERE key=?1",
                    [key],
                    |row| row.get(0),
                )
                .optional()
                .ok()
                .flatten()
        })
    }

    pub fn backup(&self) -> Result<PathBuf> {
        let connection = self.connect()?;
        connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        drop(connection);
        let destination = self
            .root
            .join("backups")
            .join(format!("coach-{}.db", Utc::now().format("%Y%m%d-%H%M%S")));
        fs::copy(&self.db_path, &destination)
            .context("Could not create encrypted database backup")?;
        Ok(destination)
    }

    fn audit<B: serde::Serialize, A: serde::Serialize>(
        &self,
        connection: &Connection,
        action: &str,
        entity_type: &str,
        entity_id: Option<&str>,
        before: Option<&B>,
        after: Option<&A>,
        reversible: bool,
    ) -> Result<()> {
        connection.execute(
            "INSERT INTO audit_events(id,action,entity_type,entity_id,before_json,after_json,reversible,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![Uuid::new_v4().to_string(), action, entity_type, entity_id, before.map(serde_json::to_string).transpose()?, after.map(serde_json::to_string).transpose()?, reversible as i64, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }
}
