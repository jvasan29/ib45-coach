use crate::{db::AppStore, models::*, path_safety::is_canonical_drive_d};
use anyhow::{Context, Result, anyhow};
use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::{Duration, Utc};
use regex::Regex;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{collections::{BTreeMap, HashSet}, fs, path::PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone)]
struct DocumentRow {
    id: String,
    title: String,
    path: String,
    subject_hint: Option<String>,
    year_hint: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnswerKeyItem {
    question_number: i64,
    choice: String,
}

pub fn library(store: &AppStore, subject_id: &str, query: &str) -> Result<ExamLibrary> {
    let connection = store.connect()?;
    let subject_name: String = connection
        .query_row("SELECT name FROM subjects WHERE id=?1", [subject_id], |row| row.get(0))
        .context("Subject was not found in this profile")?;
    let query_like = format!("%{}%", query.trim().to_lowercase());
    let mut statement = connection.prepare(
        "SELECT id,title,path,subject_hint,year_hint FROM resource_documents
         WHERE file_type='pdf' AND duplicate_of IS NULL
           AND (?1='%%' OR lower(title) LIKE ?1 OR lower(path) LIKE ?1)
         ORDER BY coalesce(year_hint,0) DESC,title LIMIT 10000",
    )?;
    let documents = statement
        .query_map([query_like], |row| {
            Ok(DocumentRow {
                id: row.get(0)?,
                title: row.get(1)?,
                path: row.get(2)?,
                subject_hint: row.get(3)?,
                year_hint: row.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .filter(|document| matches_subject_document(&subject_name, document))
        .collect::<Vec<_>>();

    let (schemes, papers): (Vec<_>, Vec<_>) = documents.into_iter().partition(|doc| is_mark_scheme(&doc.path));
    let mark_schemes = schemes
        .iter()
        .map(|doc| ExamMarkSchemeCandidate {
            id: doc.id.clone(),
            title: doc.title.clone(),
            path: doc.path.clone(),
            year_hint: doc.year_hint,
        })
        .collect::<Vec<_>>();
    let papers = papers
        .into_iter()
        .take(250)
        .map(|paper| {
            let suggested = best_mark_scheme(&paper, &schemes);
            ExamPaperCandidate {
                id: paper.id,
                title: paper.title,
                path: paper.path.clone(),
                subject_hint: paper.subject_hint,
                year_hint: paper.year_hint,
                detected_mode: detect_mode(&paper.path),
                suggested_mark_scheme_id: suggested.map(|item| item.id.clone()),
                suggested_mark_scheme_title: suggested.map(|item| item.title.clone()),
            }
        })
        .collect();
    Ok(ExamLibrary { papers, mark_schemes })
}

pub fn create_attempt(store: &AppStore, input: ExamAttemptInput) -> Result<ExamAttempt> {
    if !matches!(input.mode.as_str(), "mcq" | "theory") {
        return Err(anyhow!("Choose MCQ or theory mode"));
    }
    if !(1..=300).contains(&input.duration_minutes) {
        return Err(anyhow!("Exam duration must be between 1 and 300 minutes"));
    }
    let connection = store.connect()?;
    let subject_exists = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM subjects WHERE id=?1)",
        [&input.subject_id],
        |row| row.get::<_, bool>(0),
    )?;
    if !subject_exists {
        return Err(anyhow!("This paper is not assigned to one of your six subjects"));
    }
    let paper_path = document_path(&connection, &input.paper_document_id)?;
    if paper_path.extension().and_then(|value| value.to_str()).unwrap_or("").to_lowercase() != "pdf" {
        return Err(anyhow!("Exam Lab currently supports PDF papers only"));
    }
    let answer_key = if input.mode == "mcq" {
        input.mark_scheme_document_id.as_deref()
            .map(|id| extract_answer_key(&connection, id))
            .transpose()?
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let now = Utc::now();
    let id = Uuid::new_v4().to_string();
    connection.execute(
        "INSERT INTO exam_attempts(id,subject_id,paper_document_id,mark_scheme_document_id,mode,duration_minutes,status,started_at,ends_at,answer_key_json,created_at,updated_at)
         VALUES(?1,?2,?3,?4,?5,?6,'active',?7,?8,?9,?7,?7)",
        params![
            id,
            input.subject_id,
            input.paper_document_id,
            input.mark_scheme_document_id,
            input.mode,
            input.duration_minutes,
            now.to_rfc3339(),
            (now + Duration::minutes(input.duration_minutes)).to_rfc3339(),
            serde_json::to_string(&answer_key)?,
        ],
    )?;
    get_attempt(store, &id)
}

pub fn save_answer(store: &AppStore, input: ExamAnswerInput) -> Result<ExamAnswer> {
    let connection = store.connect()?;
    let (status, mode): (String, String) = connection.query_row(
        "SELECT status,mode FROM exam_attempts WHERE id=?1",
        [&input.attempt_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).context("Exam attempt was not found")?;
    if status != "active" {
        return Err(anyhow!("This exam has already been submitted"));
    }
    if mode == "mcq" && input.question_number.is_none() {
        return Err(anyhow!("MCQ answers require a question number"));
    }
    let now = Utc::now().to_rfc3339();
    let existing = if mode == "mcq" {
        connection.query_row(
            "SELECT id FROM exam_answers WHERE attempt_id=?1 AND question_number=?2 AND mcq_choice IS NOT NULL",
            params![input.attempt_id, input.question_number],
            |row| row.get::<_, String>(0),
        ).optional()?
    } else {
        input.answer_id.clone()
    };
    let id = existing.unwrap_or_else(|| Uuid::new_v4().to_string());
    let was_updated = connection.execute(
        "UPDATE exam_answers SET question_number=?1,page_number=?2,answer_text=?3,mcq_choice=?4,x=?5,y=?6,width=?7,height=?8,updated_at=?9 WHERE id=?10 AND attempt_id=?11",
        params![input.question_number,input.page_number,input.answer_text,input.mcq_choice,input.x,input.y,input.width,input.height,now,id,input.attempt_id],
    )? > 0;
    if !was_updated {
        connection.execute(
            "INSERT INTO exam_answers(id,attempt_id,question_number,page_number,answer_text,mcq_choice,x,y,width,height,created_at,updated_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?11)",
            params![id,input.attempt_id,input.question_number,input.page_number,input.answer_text,input.mcq_choice,input.x,input.y,input.width,input.height,now],
        )?;
    }
    connection.execute("UPDATE exam_attempts SET updated_at=?1 WHERE id=?2", params![now,input.attempt_id])?;
    read_answer(&connection, &id)
}

pub fn submit_attempt(store: &AppStore, attempt_id: &str) -> Result<ExamAttempt> {
    let connection = store.connect()?;
    let (status, mode, answer_key_json): (String, String, String) = connection.query_row(
        "SELECT status,mode,answer_key_json FROM exam_attempts WHERE id=?1",
        [attempt_id],
        |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?)),
    ).context("Exam attempt was not found")?;
    if status != "active" {
        return get_attempt(store, attempt_id);
    }
    let answer_key: Vec<AnswerKeyItem> = serde_json::from_str(&answer_key_json).unwrap_or_default();
    let now = Utc::now().to_rfc3339();
    if mode == "mcq" && !answer_key.is_empty() {
        let mut statement = connection.prepare("SELECT question_number,mcq_choice FROM exam_answers WHERE attempt_id=?1 AND mcq_choice IS NOT NULL")?;
        let answers = statement.query_map([attempt_id], |row| Ok((row.get::<_,i64>(0)?,row.get::<_,String>(1)?)))?
            .collect::<rusqlite::Result<BTreeMap<_,_>>>()?;
        let score = answer_key.iter().filter(|key| answers.get(&key.question_number).is_some_and(|answer| answer.eq_ignore_ascii_case(&key.choice))).count() as f64;
        let max_score = answer_key.len() as f64;
        connection.execute(
            "UPDATE exam_attempts SET status='graded',submitted_at=?1,score=?2,max_score=?3,percentage=?4,updated_at=?1 WHERE id=?5",
            params![now,score,max_score,score/max_score*100.0,attempt_id],
        )?;
    } else {
        connection.execute(
            "UPDATE exam_attempts SET status='awaiting_manual',submitted_at=?1,updated_at=?1 WHERE id=?2",
            params![now,attempt_id],
        )?;
    }
    get_attempt(store, attempt_id)
}

pub fn manual_score(store: &AppStore, input: ManualExamScoreInput) -> Result<ExamAttempt> {
    if input.max_score <= 0.0 || input.score < 0.0 || input.score > input.max_score {
        return Err(anyhow!("Enter a score between 0 and the maximum mark"));
    }
    let connection = store.connect()?;
    let now = Utc::now().to_rfc3339();
    let changed = connection.execute(
        "UPDATE exam_attempts SET status='graded',score=?1,max_score=?2,percentage=?3,manual_feedback=?4,submitted_at=coalesce(submitted_at,?5),updated_at=?5 WHERE id=?6 AND status<>'active'",
        params![input.score,input.max_score,input.score/input.max_score*100.0,input.feedback,now,input.attempt_id],
    )?;
    if changed == 0 {
        return Err(anyhow!("Submit the paper before entering a manual score"));
    }
    get_attempt(store, &input.attempt_id)
}

pub fn get_attempt(store: &AppStore, attempt_id: &str) -> Result<ExamAttempt> {
    let connection = store.connect()?;
    let mut attempt = connection.query_row(
        "SELECT a.id,a.subject_id,s.name,a.paper_document_id,p.title,a.mark_scheme_document_id,m.title,a.mode,a.duration_minutes,a.status,a.started_at,a.ends_at,a.submitted_at,a.score,a.max_score,a.percentage,a.manual_feedback,a.answer_key_json
         FROM exam_attempts a JOIN subjects s ON s.id=a.subject_id JOIN resource_documents p ON p.id=a.paper_document_id
         LEFT JOIN resource_documents m ON m.id=a.mark_scheme_document_id WHERE a.id=?1",
        [attempt_id],
        |row| {
            let key_json: String = row.get(17)?;
            let question_count = serde_json::from_str::<Vec<AnswerKeyItem>>(&key_json).map(|items| items.len() as i64).unwrap_or(0);
            Ok(ExamAttempt {
                id: row.get(0)?, subject_id: row.get(1)?, subject_name: row.get(2)?, paper_document_id: row.get(3)?, paper_title: row.get(4)?,
                mark_scheme_document_id: row.get(5)?, mark_scheme_title: row.get(6)?, mode: row.get(7)?, duration_minutes: row.get(8)?, status: row.get(9)?,
                started_at: row.get(10)?, ends_at: row.get(11)?, submitted_at: row.get(12)?, score: row.get(13)?, max_score: row.get(14)?, percentage: row.get(15)?, manual_feedback: row.get(16)?, question_count, answers: Vec::new(),
            })
        },
    ).context("Exam attempt was not found")?;
    let mut statement = connection.prepare(
        "SELECT id,question_number,page_number,answer_text,mcq_choice,x,y,width,height,updated_at FROM exam_answers WHERE attempt_id=?1 ORDER BY coalesce(question_number,9999),page_number,created_at",
    )?;
    attempt.answers = statement.query_map([attempt_id], map_answer)?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(attempt)
}

pub fn attempts(store: &AppStore, subject_id: Option<&str>) -> Result<Vec<ExamAttemptSummary>> {
    let connection = store.connect()?;
    let mut statement = connection.prepare(
        "SELECT a.id,a.subject_id,s.name,p.title,a.mode,a.status,a.started_at,a.ends_at,a.submitted_at,a.score,a.max_score,a.percentage
         FROM exam_attempts a JOIN subjects s ON s.id=a.subject_id JOIN resource_documents p ON p.id=a.paper_document_id
         WHERE (?1 IS NULL OR a.subject_id=?1) ORDER BY a.started_at DESC LIMIT 60",
    )?;
    Ok(statement.query_map([subject_id], |row| Ok(ExamAttemptSummary {
        id:row.get(0)?,subject_id:row.get(1)?,subject_name:row.get(2)?,paper_title:row.get(3)?,mode:row.get(4)?,status:row.get(5)?,started_at:row.get(6)?,ends_at:row.get(7)?,submitted_at:row.get(8)?,score:row.get(9)?,max_score:row.get(10)?,percentage:row.get(11)?,
    }))?.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn pdf_payload(store: &AppStore, document_id: &str) -> Result<ExamPdfPayload> {
    let connection = store.connect()?;
    let (title, path, file_type): (String,String,String) = connection.query_row(
        "SELECT title,path,file_type FROM resource_documents WHERE id=?1", [document_id],
        |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?)),
    ).context("PDF was not found in the indexed library")?;
    if file_type.to_lowercase() != "pdf" { return Err(anyhow!("The selected resource is not a PDF")); }
    let canonical = PathBuf::from(path).canonicalize()?;
    if !is_canonical_drive_d(&canonical) { return Err(anyhow!("This indexed PDF is no longer on drive D. Move it back or reindex the library.")); }
    let metadata = canonical.metadata()?;
    if metadata.len() > 80 * 1024 * 1024 { return Err(anyhow!("This PDF is larger than the 80 MB Exam Lab limit")); }
    Ok(ExamPdfPayload { document_id:document_id.to_string(),title,data_base64:STANDARD.encode(fs::read(canonical)?) })
}

fn document_path(connection: &rusqlite::Connection, id: &str) -> Result<PathBuf> {
    let path: String = connection.query_row("SELECT path FROM resource_documents WHERE id=?1", [id], |row| row.get(0))?;
    let canonical = PathBuf::from(path).canonicalize()?;
    if !is_canonical_drive_d(&canonical) { return Err(anyhow!("This indexed document is no longer on drive D. Move it back or reindex the library.")); }
    Ok(canonical)
}

fn extract_answer_key(connection: &rusqlite::Connection, document_id: &str) -> Result<Vec<AnswerKeyItem>> {
    let mut statement = connection.prepare("SELECT body FROM resource_chunks WHERE document_id=?1 ORDER BY chunk_index")?;
    let mut text = statement.query_map([document_id], |row| row.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?.join("\n");
    if text.trim().is_empty() {
        let path = document_path(connection, document_id)?;
        text = pdf_extract::extract_text(path).context("Could not extract the mark scheme text")?;
    }
    Ok(parse_answer_key(&text))
}

fn parse_answer_key(text: &str) -> Vec<AnswerKeyItem> {
    let pattern = Regex::new(r"(?i)\b(\d{1,3})\s*[\).:\-]?\s*([A-D])\b").expect("valid answer key regex");
    let mut candidates = BTreeMap::new();
    for captures in pattern.captures_iter(text) {
        let Some(number) = captures.get(1).and_then(|value| value.as_str().parse::<i64>().ok()) else { continue; };
        if !(1..=150).contains(&number) { continue; }
        candidates.entry(number).or_insert_with(|| captures[2].to_uppercase());
    }
    let max = candidates.keys().copied().max().unwrap_or(0);
    if max < 3 || max > 100 { return Vec::new(); }
    let covered = (1..=max).filter(|number| candidates.contains_key(number)).count() as f64 / max as f64;
    if covered < 0.68 || !candidates.contains_key(&1) { return Vec::new(); }
    candidates.into_iter().map(|(question_number,choice)| AnswerKeyItem { question_number,choice }).collect()
}

fn read_answer(connection: &rusqlite::Connection, id: &str) -> Result<ExamAnswer> {
    Ok(connection.query_row(
        "SELECT id,question_number,page_number,answer_text,mcq_choice,x,y,width,height,updated_at FROM exam_answers WHERE id=?1",
        [id], map_answer,
    )?)
}

fn map_answer(row: &rusqlite::Row<'_>) -> rusqlite::Result<ExamAnswer> {
    Ok(ExamAnswer { id:row.get(0)?,question_number:row.get(1)?,page_number:row.get(2)?,answer_text:row.get(3)?,mcq_choice:row.get(4)?,x:row.get(5)?,y:row.get(6)?,width:row.get(7)?,height:row.get(8)?,updated_at:row.get(9)? })
}

fn is_mark_scheme(path: &str) -> bool {
    let lower = path.to_lowercase().replace('\\', "/");
    lower.contains("mark scheme") || lower.contains("markscheme") || lower.contains("mark_scheme") || lower.contains("/ms/") || lower.contains("_ms.") || lower.contains("-ms.") || lower.ends_with(" ms.pdf")
}

fn detect_mode(path: &str) -> String {
    let lower = path.to_lowercase();
    if lower.contains("multiple choice") || lower.contains("multiple_choice") || lower.contains("mcq") || lower.contains("paper 1a") || lower.contains("paper_1a") || lower.contains("p1a") { "mcq" } else { "theory" }.to_string()
}

fn matches_subject_document(subject_name: &str, document: &DocumentRow) -> bool {
    let subject = subject_name.to_lowercase();
    let title = document.title.to_lowercase().replace(['_', '-'], " ");
    if subject.contains("math") {
        return title.contains("math");
    }
    if subject.contains("business") {
        return title.contains("business") || title.contains("management");
    }
    if subject.contains("computer science") {
        return title.contains("computer science") || title.contains("computing");
    }
    if subject.contains("english") {
        return title.contains("english a") || title.contains("english language") || title.contains("english literature");
    }
    if subject.contains("physics") {
        return title.contains("physics");
    }
    if subject.contains("french") {
        // Avoid papers from other subjects that merely have a French translation.
        return title.contains("french b") || title.starts_with("french b ") || title.contains("french ab initio");
    }
    let tokens = subject.split_whitespace().filter(|token| token.len() >= 4).collect::<Vec<_>>();
    !tokens.is_empty() && tokens.iter().all(|token| title.contains(token))
}

fn identity_tokens(path: &str) -> HashSet<String> {
    let ignored = ["pdf","mark","scheme","markscheme","ms","question","questions","paper","papers","qp","answer","answers"];
    path.to_lowercase().split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| token.len() > 1 && !ignored.contains(token))
        .map(str::to_string).collect()
}

fn pairing_score(paper: &DocumentRow, scheme: &DocumentRow) -> f64 {
    let a = identity_tokens(&paper.path); let b = identity_tokens(&scheme.path);
    if a.is_empty() || b.is_empty() { return 0.0; }
    let shared = a.intersection(&b).count() as f64;
    let union = a.union(&b).count() as f64;
    let year_bonus = if paper.year_hint.is_some() && paper.year_hint == scheme.year_hint { 0.18 } else { 0.0 };
    shared / union + year_bonus
}

fn best_mark_scheme<'a>(paper: &DocumentRow, schemes: &'a [DocumentRow]) -> Option<&'a DocumentRow> {
    schemes.iter().map(|scheme| (pairing_score(paper,scheme),scheme))
        .filter(|(score,_)| *score >= 0.48)
        .max_by(|a,b| a.0.total_cmp(&b.0)).map(|(_,scheme)| scheme)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dense_mcq_answer_keys() {
        let result = parse_answer_key("1 A  2 B  3 D  4 C  5 A");
        assert_eq!(result.len(), 5);
        assert_eq!(result[2].choice, "D");
    }

    #[test]
    fn rejects_sparse_question_text_as_an_answer_key() {
        assert!(parse_answer_key("Question 1 a student writes. Later 14 B appears.").is_empty());
    }

    #[test]
    fn pairs_question_papers_with_matching_mark_schemes() {
        let paper = DocumentRow { id:"p".into(),title:"paper".into(),path:r"D:\Physics\m24_physics_p1_qp.pdf".into(),subject_hint:None,year_hint:Some(2024) };
        let scheme = DocumentRow { id:"m".into(),title:"scheme".into(),path:r"D:\Physics\m24_physics_p1_ms.pdf".into(),subject_hint:None,year_hint:Some(2024) };
        assert!(pairing_score(&paper,&scheme) > 0.7);
    }

    #[test]
    fn subject_filter_uses_filename_instead_of_broad_archive_folders() {
        let computer_science = DocumentRow { id:"cs".into(),title:"Computer_science_paper_1_HL".into(),path:r"D:\Group 5 - Mathematics and Computer Science\Computer_science_paper_1_HL.pdf".into(),subject_hint:Some("Mathematics".into()),year_hint:Some(2010) };
        assert!(!matches_subject_document("Math AA", &computer_science));
        assert!(matches_subject_document("Computer Science", &computer_science));
    }

    #[test]
    fn french_filter_excludes_translated_math_papers() {
        let translated_math = DocumentRow { id:"m".into(),title:"Mathematics_paper_1_HL_French".into(),path:"D:/paper.pdf".into(),subject_hint:Some("Mathematics".into()),year_hint:Some(2010) };
        assert!(!matches_subject_document("French B", &translated_math));
    }
}
