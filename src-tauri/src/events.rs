use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::Manager;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventRecord {
    pub id: u64,
    pub kind: String,
    pub label: String,
    pub detail: String,
}

pub fn serialize_record(record: &EventRecord) -> String {
    serde_json::to_string(record).unwrap_or_default()
}

pub fn parse_record(line: &str) -> Option<EventRecord> {
    serde_json::from_str(line.trim()).ok()
}

pub fn append_record(path: &Path, record: &EventRecord) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|error| format!("Cannot create app data directory: {error}"))?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("Cannot open events file: {error}"))?;
    writeln!(file, "{}", serialize_record(record))
        .map_err(|error| format!("Cannot write event: {error}"))?;
    Ok(())
}

pub fn load_records(path: &Path, limit: usize) -> Vec<EventRecord> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return Vec::new(),
    };

    let mut records: Vec<EventRecord> = content.lines().filter_map(parse_record).collect();
    records.reverse();
    records.truncate(limit);
    records
}

pub fn events_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|dir| dir.join("events.jsonl"))
        .map_err(|error| format!("Cannot resolve app data directory: {error}"))
}

#[tauri::command]
pub fn append_event(
    app: tauri::AppHandle,
    kind: String,
    label: String,
    detail: String,
) -> Result<EventRecord, String> {
    let path = events_path(&app)?;
    let id = load_records(&path, usize::MAX).len() as u64;
    let record = EventRecord {
        id,
        kind,
        label,
        detail,
    };
    append_record(&path, &record)?;
    Ok(record)
}

#[tauri::command]
pub fn load_events(app: tauri::AppHandle, limit: u32) -> Vec<EventRecord> {
    match events_path(&app) {
        Ok(path) => load_records(&path, limit as usize),
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: u64) -> EventRecord {
        EventRecord {
            id,
            kind: "command".to_string(),
            label: "navigate".to_string(),
            detail: format!("detail-{id}"),
        }
    }

    #[test]
    fn serializes_and_parses_a_record_round_trip() {
        let line = serialize_record(&record(3));
        assert!(!line.contains('\n'));
        assert_eq!(parse_record(&line), Some(record(3)));
    }

    #[test]
    fn parse_skips_malformed_lines() {
        assert_eq!(parse_record("not json"), None);
        assert_eq!(parse_record(""), None);
    }

    #[test]
    fn appends_and_loads_the_recent_tail_newest_first() {
        let path =
            std::env::temp_dir().join(format!("synth-fs032-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&path);

        for id in 0..3 {
            append_record(&path, &record(id)).unwrap();
        }
        // a malformed line is tolerated
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"garbage\n")
            .unwrap();

        let recent = load_records(&path, 2);
        assert_eq!(recent.iter().map(|r| r.id).collect::<Vec<_>>(), vec![2, 1]);

        assert!(load_records(Path::new("/no/such/synth/events.jsonl"), 5).is_empty());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn serializes_record_in_camel_case() {
        let serialized = serde_json::to_value(record(1)).unwrap();
        assert_eq!(serialized["id"], 1);
        assert_eq!(serialized["kind"], "command");
        assert_eq!(serialized["label"], "navigate");
    }
}
