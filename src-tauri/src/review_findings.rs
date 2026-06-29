use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::Manager;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewFinding {
    pub id: u64,
    pub kind: String,
    pub subject: String,
    pub finding: String,
}

pub fn is_valid_review_kind(kind: &str) -> bool {
    matches!(kind, "diff" | "requirements")
}

pub fn serialize_record(record: &ReviewFinding) -> String {
    serde_json::to_string(record).unwrap_or_default()
}

pub fn parse_record(line: &str) -> Option<ReviewFinding> {
    serde_json::from_str(line.trim()).ok()
}

pub fn append_record(path: &Path, record: &ReviewFinding) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|error| format!("Cannot create app data directory: {error}"))?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("Cannot open review findings file: {error}"))?;
    writeln!(file, "{}", serialize_record(record))
        .map_err(|error| format!("Cannot write review finding: {error}"))?;
    Ok(())
}

pub fn load_records(path: &Path, limit: usize) -> Vec<ReviewFinding> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return Vec::new(),
    };

    let mut records: Vec<ReviewFinding> = content.lines().filter_map(parse_record).collect();
    records.reverse();
    records.truncate(limit);
    records
}

pub fn review_findings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|dir| dir.join("review-findings.jsonl"))
        .map_err(|error| format!("Cannot resolve app data directory: {error}"))
}

#[tauri::command]
pub fn capture_review_finding(
    app: tauri::AppHandle,
    kind: String,
    subject: String,
    finding: String,
) -> Result<ReviewFinding, String> {
    if !is_valid_review_kind(&kind) {
        return Err("Unknown review kind.".to_string());
    }
    let finding = finding.trim();
    if finding.is_empty() {
        return Err("Nothing to capture.".to_string());
    }

    let path = review_findings_path(&app)?;
    let id = load_records(&path, usize::MAX).len() as u64;
    let record = ReviewFinding {
        id,
        kind,
        subject: subject.trim().to_string(),
        finding: finding.to_string(),
    };
    append_record(&path, &record)?;
    Ok(record)
}

#[tauri::command]
pub fn list_review_findings(app: tauri::AppHandle, limit: u32) -> Vec<ReviewFinding> {
    match review_findings_path(&app) {
        Ok(path) => load_records(&path, limit as usize),
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(id: u64, kind: &str) -> ReviewFinding {
        ReviewFinding {
            id,
            kind: kind.to_string(),
            subject: "working tree".to_string(),
            finding: "a concrete concern".to_string(),
        }
    }

    #[test]
    fn validates_review_kind() {
        assert!(is_valid_review_kind("diff"));
        assert!(is_valid_review_kind("requirements"));
        assert!(!is_valid_review_kind("bogus"));
        assert!(!is_valid_review_kind(""));
    }

    #[test]
    fn serializes_review_finding_in_camel_case() {
        let serialized = serde_json::to_value(finding(2, "requirements")).unwrap();
        assert_eq!(serialized["id"], 2);
        assert_eq!(serialized["kind"], "requirements");
        assert_eq!(serialized["subject"], "working tree");
        assert_eq!(serialized["finding"], "a concrete concern");
    }

    #[test]
    fn round_trips_serialize_and_parse() {
        let record = finding(5, "diff");
        let line = serialize_record(&record);
        assert_eq!(parse_record(&line), Some(record));
        assert_eq!(parse_record("not json"), None);
    }

    #[test]
    fn appends_and_loads_recent_tail_newest_first_skipping_malformed() {
        let path = std::env::temp_dir().join(format!("synth-fs052-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&path);

        append_record(&path, &finding(0, "diff")).unwrap();
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"not json\n")
            .unwrap();
        append_record(&path, &finding(1, "requirements")).unwrap();

        let recent = load_records(&path, 10);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].id, 1);
        assert_eq!(recent[1].id, 0);
        assert_eq!(load_records(&path, 1).len(), 1);
        assert!(load_records(Path::new("/no/such/synth/review-findings.jsonl"), 5).is_empty());

        let _ = std::fs::remove_file(&path);
    }
}
