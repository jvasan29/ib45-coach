use crate::{
    db::AppStore,
    models::{IndexStatus, ResourceResult},
};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use quick_xml::{Reader, events::Event};
use rusqlite::{OptionalExtension, params};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::Duration,
};
use uuid::Uuid;
use walkdir::WalkDir;

const METADATA_BATCH_SIZE: usize = 200;

#[derive(Clone)]
pub struct ResourceIndexer {
    inner: Arc<IndexerInner>,
}

struct IndexerInner {
    running: AtomicBool,
    paused: AtomicBool,
    cancel: AtomicBool,
    scanned: AtomicU64,
    indexed: AtomicU64,
    skipped: AtomicU64,
    failed: AtomicU64,
    current_file: RwLock<String>,
    started_at: RwLock<Option<String>>,
}

impl Default for ResourceIndexer {
    fn default() -> Self {
        Self {
            inner: Arc::new(IndexerInner {
                running: AtomicBool::new(false),
                paused: AtomicBool::new(false),
                cancel: AtomicBool::new(false),
                scanned: AtomicU64::new(0),
                indexed: AtomicU64::new(0),
                skipped: AtomicU64::new(0),
                failed: AtomicU64::new(0),
                current_file: RwLock::new(String::new()),
                started_at: RwLock::new(None),
            }),
        }
    }
}

impl ResourceIndexer {
    pub fn start(&self, store: AppStore, paths: Vec<String>) -> Result<()> {
        if self.inner.running.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        self.inner.cancel.store(false, Ordering::SeqCst);
        self.inner.paused.store(false, Ordering::SeqCst);
        for counter in [
            &self.inner.scanned,
            &self.inner.indexed,
            &self.inner.skipped,
            &self.inner.failed,
        ] {
            counter.store(0, Ordering::SeqCst);
        }
        *self.inner.started_at.write().unwrap() = Some(Utc::now().to_rfc3339());
        let control = self.clone();
        thread::spawn(move || {
            let roots = if paths.is_empty() {
                vec![
                    r"D:\IB Past Papers".to_string(),
                    r"D:\IB Mark Schemes Output".to_string(),
                    r"D:\IB Calculus Output".to_string(),
                    r"D:\STUDY RESOURCES".to_string(),
                ]
            } else {
                paths
            };
            let mut files = Vec::new();
            for root in roots {
                if control.inner.cancel.load(Ordering::SeqCst) {
                    break;
                }
                let path = PathBuf::from(root);
                if !path.exists() {
                    continue;
                }
                for entry in WalkDir::new(path)
                    .follow_links(false)
                    .into_iter()
                    .filter_map(Result::ok)
                {
                    if !entry.file_type().is_file() || !supported(entry.path()) {
                        continue;
                    }
                    files.push(entry.path().to_path_buf());
                }
            }
            sort_newest_first(&mut files);
            control
                .inner
                .scanned
                .store(files.len() as u64, Ordering::SeqCst);
            *control.inner.current_file.write().unwrap() =
                format!("Registering metadata for {} files", files.len());
            if let Err(error) = register_metadata_catalog(&store, &files, &control) {
                control.inner.failed.fetch_add(1, Ordering::SeqCst);
                let _ = append_error_log(
                    &store,
                    &store.root,
                    &format!("Metadata catalog failed: {error}"),
                );
            }
            for file in files {
                if control.inner.cancel.load(Ordering::SeqCst) {
                    break;
                }
                while control.inner.paused.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(300));
                    if control.inner.cancel.load(Ordering::SeqCst) {
                        break;
                    }
                }
                *control.inner.current_file.write().unwrap() = file.display().to_string();
                if fs2::available_space(&store.root).unwrap_or(u64::MAX) < 10 * 1024 * 1024 * 1024 {
                    control.inner.paused.store(true, Ordering::SeqCst);
                    let _ = store.set_setting(
                        "index_pause_reason",
                        "Indexing paused because drive D has less than 10 GB free.",
                    );
                    break;
                }
                match index_file(&store, &file) {
                    Ok(IndexOutcome::Indexed) => {
                        control.inner.indexed.fetch_add(1, Ordering::SeqCst);
                    }
                    Ok(IndexOutcome::Skipped) => {
                        control.inner.skipped.fetch_add(1, Ordering::SeqCst);
                    }
                    Err(error) => {
                        control.inner.failed.fetch_add(1, Ordering::SeqCst);
                        let _ = append_error_log(&store, &file, &error.to_string());
                    }
                }
            }
            control.inner.running.store(false, Ordering::SeqCst);
            *control.inner.current_file.write().unwrap() = String::new();
            let _ = store.set_setting("last_index_completed_at", &Utc::now().to_rfc3339());
        });
        Ok(())
    }

    pub fn set_paused(&self, paused: bool) {
        self.inner.paused.store(paused, Ordering::SeqCst);
    }

    pub fn cancel(&self) {
        self.inner.cancel.store(true, Ordering::SeqCst);
    }

    pub fn status(&self) -> IndexStatus {
        IndexStatus {
            running: self.inner.running.load(Ordering::SeqCst),
            paused: self.inner.paused.load(Ordering::SeqCst),
            scanned: self.inner.scanned.load(Ordering::SeqCst),
            indexed: self.inner.indexed.load(Ordering::SeqCst),
            skipped: self.inner.skipped.load(Ordering::SeqCst),
            failed: self.inner.failed.load(Ordering::SeqCst),
            current_file: self.inner.current_file.read().unwrap().clone(),
            started_at: self.inner.started_at.read().unwrap().clone(),
        }
    }
}

fn sort_newest_first(files: &mut [PathBuf]) {
    files.sort_by(|left, right| {
        let left_text = left.display().to_string();
        let right_text = right.display().to_string();
        let left_year = infer_metadata(&left_text).1.unwrap_or(0);
        let right_year = infer_metadata(&right_text).1.unwrap_or(0);
        right_year.cmp(&left_year).then_with(|| right.cmp(left))
    });
}

fn register_metadata_catalog(
    store: &AppStore,
    files: &[PathBuf],
    control: &ResourceIndexer,
) -> Result<()> {
    let mut connection = store.connect()?;
    for (batch_index, batch) in files.chunks(METADATA_BATCH_SIZE).enumerate() {
        while control.inner.paused.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(300));
            if control.inner.cancel.load(Ordering::SeqCst) {
                return Ok(());
            }
        }
        if control.inner.cancel.load(Ordering::SeqCst) {
            break;
        }

        // Keep write locks brief. A paused indexer must never retain a transaction,
        // and foreground saves get a chance to proceed between metadata batches.
        let transaction = connection.transaction()?;
        for (offset, path) in batch.iter().enumerate() {
            if control.inner.cancel.load(Ordering::SeqCst) {
                break;
            }
            let index = batch_index * METADATA_BATCH_SIZE + offset;
            if index % METADATA_BATCH_SIZE == 0 {
                *control.inner.current_file.write().unwrap() =
                    format!("Registering paper metadata · {} / {}", index, files.len());
            }
            let Ok(metadata) = path.metadata() else {
                continue;
            };
            let modified: DateTime<Utc> = metadata
                .modified()
                .map(DateTime::<Utc>::from)
                .unwrap_or_else(|_| Utc::now());
            let path_text = path.display().to_string();
            let title = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("Untitled resource")
                .to_string();
            let extension = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("")
                .to_lowercase();
            let (subject_hint, year_hint) = infer_metadata(&path_text);
            transaction.execute(
                "INSERT INTO resource_documents(id,path,title,file_type,size_bytes,modified_at,subject_hint,year_hint,extraction_state,indexed_at)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,'metadata_pending',?9)
                 ON CONFLICT(path) DO UPDATE SET
                   title=excluded.title,file_type=excluded.file_type,subject_hint=excluded.subject_hint,year_hint=excluded.year_hint,
                   sha256=CASE WHEN resource_documents.size_bytes<>excluded.size_bytes OR resource_documents.modified_at<>excluded.modified_at THEN NULL ELSE resource_documents.sha256 END,
                   duplicate_of=CASE WHEN resource_documents.size_bytes<>excluded.size_bytes OR resource_documents.modified_at<>excluded.modified_at THEN NULL ELSE resource_documents.duplicate_of END,
                   extraction_error=CASE WHEN resource_documents.size_bytes<>excluded.size_bytes OR resource_documents.modified_at<>excluded.modified_at THEN NULL ELSE resource_documents.extraction_error END,
                   extraction_state=CASE WHEN resource_documents.size_bytes<>excluded.size_bytes OR resource_documents.modified_at<>excluded.modified_at THEN 'metadata_pending' ELSE resource_documents.extraction_state END,
                   size_bytes=excluded.size_bytes,modified_at=excluded.modified_at,indexed_at=excluded.indexed_at",
                params![Uuid::new_v4().to_string(),path_text,title,extension,metadata.len() as i64,modified.to_rfc3339(),subject_hint,year_hint,Utc::now().to_rfc3339()],
            )?;
        }
        transaction.commit()?;
        thread::sleep(Duration::from_millis(15));
    }
    Ok(())
}

enum IndexOutcome {
    Indexed,
    Skipped,
}

fn supported(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_lowercase()
            .as_str(),
        "pdf"
            | "docx"
            | "txt"
            | "md"
            | "html"
            | "htm"
            | "csv"
            | "tsv"
            | "png"
            | "jpg"
            | "jpeg"
            | "webp"
    )
}

fn index_file(store: &AppStore, path: &Path) -> Result<IndexOutcome> {
    let metadata = path.metadata()?;
    let modified: DateTime<Utc> = metadata
        .modified()
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(|_| Utc::now());
    let path_text = path.display().to_string();
    let connection = store.connect()?;
    let unchanged = connection
        .query_row(
            "SELECT size_bytes=?2 AND modified_at=?3 AND extraction_state<>'metadata_pending' FROM resource_documents WHERE path=?1",
            params![path_text, metadata.len() as i64, modified.to_rfc3339()],
            |row| row.get::<_, bool>(0),
        )
        .optional()?
        .unwrap_or(false);
    if unchanged {
        return Ok(IndexOutcome::Skipped);
    }

    let sha256 = hash_file(path)?;
    let duplicate_of: Option<String> = connection
        .query_row(
            "SELECT id FROM resource_documents WHERE sha256=?1 AND path<>?2 LIMIT 1",
            params![sha256, path_text],
            |row| row.get(0),
        )
        .optional()?;
    let id: String = connection
        .query_row(
            "SELECT id FROM resource_documents WHERE path=?1",
            [&path_text],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let title = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Untitled resource")
        .to_string();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_lowercase();
    let (subject_hint, year_hint) = infer_metadata(&path_text);
    if let Some(original) = duplicate_of {
        connection.execute(
            "INSERT INTO resource_documents(id,path,title,file_type,size_bytes,modified_at,sha256,duplicate_of,subject_hint,year_hint,extraction_state,indexed_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,'duplicate',?11)
             ON CONFLICT(path) DO UPDATE SET title=excluded.title,file_type=excluded.file_type,size_bytes=excluded.size_bytes,modified_at=excluded.modified_at,sha256=excluded.sha256,duplicate_of=excluded.duplicate_of,subject_hint=excluded.subject_hint,year_hint=excluded.year_hint,extraction_state='duplicate',indexed_at=excluded.indexed_at",
            params![id,path_text,title,extension,metadata.len() as i64,modified.to_rfc3339(),sha256,original,subject_hint,year_hint,Utc::now().to_rfc3339()],
        )?;
        return Ok(IndexOutcome::Indexed);
    }

    let (text, state, extraction_error) = match extract_text(path, &extension) {
        Ok(value) if value.trim().len() >= 40 => (value, "ready", None),
        Ok(_) if matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp" | "pdf") => {
            (String::new(), "ocr_pending", None)
        }
        Ok(_) => (String::new(), "metadata", None),
        Err(error) => (String::new(), "failed", Some(error.to_string())),
    };
    let mut connection = store.connect()?;
    let transaction = connection.transaction()?;
    transaction.execute(
        "INSERT INTO resource_documents(id,path,title,file_type,size_bytes,modified_at,sha256,duplicate_of,subject_hint,year_hint,extraction_state,extraction_error,indexed_at)
         VALUES(?1,?2,?3,?4,?5,?6,?7,NULL,?8,?9,?10,?11,?12)
         ON CONFLICT(path) DO UPDATE SET title=excluded.title,file_type=excluded.file_type,size_bytes=excluded.size_bytes,modified_at=excluded.modified_at,sha256=excluded.sha256,duplicate_of=NULL,subject_hint=excluded.subject_hint,year_hint=excluded.year_hint,extraction_state=excluded.extraction_state,extraction_error=excluded.extraction_error,indexed_at=excluded.indexed_at",
        params![id,path_text,title,extension,metadata.len() as i64,modified.to_rfc3339(),sha256,subject_hint,year_hint,state,extraction_error,Utc::now().to_rfc3339()],
    )?;
    transaction.execute("DELETE FROM resource_fts WHERE document_id=?1", [&id])?;
    transaction.execute("DELETE FROM resource_chunks WHERE document_id=?1", [&id])?;
    for (index, chunk) in chunk_text(&text, 1800, 180).into_iter().enumerate() {
        let chunk_id = Uuid::new_v4().to_string();
        transaction.execute(
            "INSERT INTO resource_chunks(id,document_id,chunk_index,body) VALUES(?1,?2,?3,?4)",
            params![chunk_id, id, index as i64, chunk],
        )?;
        transaction.execute(
            "INSERT INTO resource_fts(chunk_id,document_id,title,body,path) VALUES(?1,?2,?3,?4,?5)",
            params![chunk_id, id, title, chunk, path_text],
        )?;
    }
    transaction.commit()?;
    Ok(IndexOutcome::Indexed)
}

fn hash_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 1024 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn extract_text(path: &Path, extension: &str) -> Result<String> {
    match extension {
        "pdf" => pdf_extract::extract_text(path).context("PDF text extraction failed"),
        "docx" => extract_docx(path),
        "txt" | "md" | "csv" | "tsv" => read_limited_text(path, 12 * 1024 * 1024),
        "html" | "htm" => {
            read_limited_text(path, 8 * 1024 * 1024).map(|value| strip_markup(&value))
        }
        "png" | "jpg" | "jpeg" | "webp" => run_tesseract(path),
        _ => Ok(String::new()),
    }
}

fn read_limited_text(path: &Path, max_bytes: usize) -> Result<String> {
    let file = File::open(path)?;
    let mut bytes = Vec::new();
    file.take(max_bytes as u64).read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn extract_docx(path: &Path) -> Result<String> {
    let file = File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")?
        .read_to_string(&mut xml)?;
    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(true);
    let mut output = String::new();
    loop {
        match reader.read_event() {
            Ok(Event::Text(text)) => {
                let decoded = text.decode()?;
                output.push_str(&decoded);
                output.push(' ');
            }
            Ok(Event::Eof) => break,
            Err(error) => return Err(error.into()),
            _ => {}
        }
    }
    Ok(output)
}

fn run_tesseract(path: &Path) -> Result<String> {
    let output = Command::new("tesseract")
        .arg(path)
        .arg("stdout")
        .arg("-l")
        .arg("eng")
        .output()
        .context("Tesseract is not installed")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            String::from_utf8_lossy(&output.stderr).into_owned()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn strip_markup(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut in_tag = false;
    for character in value.chars() {
        match character {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                result.push(' ');
            }
            _ if !in_tag => result.push(character),
            _ => {}
        }
    }
    result
}

fn chunk_text(value: &str, size: usize, overlap: usize) -> Vec<String> {
    let characters: Vec<char> = value.chars().collect();
    if characters.is_empty() {
        return Vec::new();
    }
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < characters.len() {
        let end = (start + size).min(characters.len());
        chunks.push(characters[start..end].iter().collect::<String>());
        if end == characters.len() {
            break;
        }
        start = end.saturating_sub(overlap);
    }
    chunks
}

fn infer_metadata(path: &str) -> (Option<String>, Option<i64>) {
    let lower = path.to_lowercase();
    let subjects = [
        ("mathemat", "Mathematics"),
        ("physics", "Physics"),
        ("chemistry", "Chemistry"),
        ("biology", "Biology"),
        ("economics", "Economics"),
        ("business", "Business Management"),
        ("history", "History"),
        ("geography", "Geography"),
        ("psychology", "Psychology"),
        ("computer science", "Computer Science"),
        ("english", "English"),
        ("french", "French"),
        ("spanish", "Spanish"),
        ("chinese", "Chinese"),
        ("tok", "Theory of Knowledge"),
    ];
    let subject = subjects
        .iter()
        .find(|(needle, _)| lower.contains(needle))
        .map(|(_, name)| name.to_string());
    let year = lower
        .split(|character: char| !character.is_ascii_digit())
        .find_map(|token| {
            if token.len() == 4 {
                token
                    .parse::<i64>()
                    .ok()
                    .filter(|value| (2000..=2100).contains(value))
            } else {
                None
            }
        });
    (subject, year)
}

pub fn search(store: &AppStore, query: &str, limit: i64) -> Result<Vec<ResourceResult>> {
    let connection = store.connect()?;
    if query.trim().is_empty() {
        let mut statement = connection.prepare(
            "SELECT id,title,path,file_type,size_bytes,subject_hint,year_hint,extraction_state,'' FROM resource_documents ORDER BY indexed_at DESC LIMIT ?1",
        )?;
        let rows = statement.query_map([limit], map_resource_row)?;
        return Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?);
    }
    let terms = query
        .split_whitespace()
        .filter(|term| !term.is_empty())
        .map(|term| format!("\"{}\"", term.replace('"', "")))
        .collect::<Vec<_>>()
        .join(" OR ");
    let mut statement = connection.prepare(
        "SELECT d.id,d.title,d.path,d.file_type,d.size_bytes,d.subject_hint,d.year_hint,d.extraction_state,
                snippet(resource_fts,3,'<mark>','</mark>',' … ',20),bm25(resource_fts)
         FROM resource_fts JOIN resource_documents d ON d.id=resource_fts.document_id
         WHERE resource_fts MATCH ?1 GROUP BY d.id ORDER BY bm25(resource_fts) LIMIT ?2",
    )?;
    let rows = statement.query_map(params![terms, limit], |row| {
        Ok(ResourceResult {
            id: row.get(0)?,
            title: row.get(1)?,
            path: row.get(2)?,
            file_type: row.get(3)?,
            size_bytes: row.get(4)?,
            subject_hint: row.get(5)?,
            year_hint: row.get(6)?,
            extraction_state: row.get(7)?,
            snippet: row.get(8)?,
            score: row.get::<_, f64>(9).unwrap_or(0.0).abs(),
        })
    })?;
    let results = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    if !results.is_empty() {
        return Ok(results);
    }
    let like = format!("%{}%", query);
    let mut fallback = connection.prepare(
        "SELECT id,title,path,file_type,size_bytes,subject_hint,year_hint,extraction_state,'' FROM resource_documents WHERE title LIKE ?1 OR path LIKE ?1 ORDER BY indexed_at DESC LIMIT ?2",
    )?;
    Ok(fallback
        .query_map(params![like, limit], map_resource_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

fn map_resource_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ResourceResult> {
    Ok(ResourceResult {
        id: row.get(0)?,
        title: row.get(1)?,
        path: row.get(2)?,
        file_type: row.get(3)?,
        size_bytes: row.get(4)?,
        subject_hint: row.get(5)?,
        year_hint: row.get(6)?,
        extraction_state: row.get(7)?,
        snippet: row.get(8)?,
        score: 0.0,
    })
}

fn append_error_log(store: &AppStore, path: &Path, error: &str) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(store.root.join("logs").join("index-errors.log"))?;
    writeln!(
        file,
        "{}\t{}\t{}",
        Utc::now().to_rfc3339(),
        path.display(),
        error
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_overlap_without_losing_text() {
        let value = "a".repeat(5000);
        let chunks = chunk_text(&value, 1800, 180);
        assert_eq!(chunks.len(), 3);
        assert!(chunks.iter().all(|chunk| !chunk.is_empty()));
    }

    #[test]
    fn metadata_is_inferred_conservatively() {
        let (subject, year) = infer_metadata(r"D:\IB Past Papers\Physics\2024.pdf");
        assert_eq!(subject.as_deref(), Some("Physics"));
        assert_eq!(year, Some(2024));
    }

    #[test]
    fn extraction_queue_prioritizes_newest_exam_sessions() {
        let mut files = vec![
            PathBuf::from(r"D:\IB Past Papers\2010 Examination Session\old.pdf"),
            PathBuf::from(r"D:\IB Past Papers\2025 Examination Session\new.pdf"),
            PathBuf::from(r"D:\IB Past Papers\2023 Examination Session\middle.pdf"),
        ];
        sort_newest_first(&mut files);
        assert!(files[0].display().to_string().contains("2025"));
        assert!(files[2].display().to_string().contains("2010"));
    }
}
