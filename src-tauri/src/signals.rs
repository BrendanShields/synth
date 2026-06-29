use serde::Serialize;

use crate::events::EventRecord;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Signal {
    pub kind: String,
    pub summary: String,
    pub count: u32,
}

fn is_command_failure(detail: &str) -> bool {
    let lower = detail.to_lowercase();
    lower.contains("timed out")
        || (detail.contains("[exit ") && !detail.contains("[exit 0]"))
}

fn is_amendment(detail: &str) -> bool {
    detail.contains("amendments/") || detail.to_lowercase().contains("amendment")
}

pub fn detect_signals(events: &[EventRecord]) -> Vec<Signal> {
    let mut errors = 0u32;
    let mut failures = 0u32;
    let mut amendments = 0u32;

    for event in events {
        if event.kind == "error" {
            errors += 1;
        }
        if is_command_failure(&event.detail) {
            failures += 1;
        }
        if is_amendment(&event.detail) {
            amendments += 1;
        }
    }

    let mut signals = Vec::new();
    if errors >= 2 {
        signals.push(Signal {
            kind: "repeated-errors".to_string(),
            summary: format!("{errors} errors recorded recently"),
            count: errors,
        });
    }
    if failures >= 1 {
        signals.push(Signal {
            kind: "command-failure".to_string(),
            summary: format!("{failures} command failure(s) recorded"),
            count: failures,
        });
    }
    if amendments >= 1 {
        signals.push(Signal {
            kind: "amendment".to_string(),
            summary: format!("{amendments} spec amendment(s) recorded"),
            count: amendments,
        });
    }
    signals
}

#[tauri::command]
pub fn improvement_signals(app: tauri::AppHandle) -> Vec<Signal> {
    detect_signals(&crate::events::load_events(app, 200))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(kind: &str, detail: &str) -> EventRecord {
        EventRecord {
            id: 0,
            kind: kind.to_string(),
            label: "x".to_string(),
            detail: detail.to_string(),
        }
    }

    #[test]
    fn detects_repeated_errors() {
        let events = vec![ev("error", "a"), ev("command", "b"), ev("error", "c")];
        let signals = detect_signals(&events);
        let errors = signals.iter().find(|s| s.kind == "repeated-errors").unwrap();
        assert_eq!(errors.count, 2);
    }

    #[test]
    fn detects_command_failures() {
        let events = vec![
            ev("command", "ran ok\n[exit 0]"),
            ev("command", "boom\n[exit 1]"),
            ev("command", "[timed out after 30s]"),
        ];
        let signals = detect_signals(&events);
        assert_eq!(
            signals.iter().find(|s| s.kind == "command-failure").unwrap().count,
            2
        );
    }

    #[test]
    fn detects_amendments() {
        let events = vec![ev("command", "requested write docs/specs/FS-005/amendments/AMD-001.md")];
        assert!(detect_signals(&events).iter().any(|s| s.kind == "amendment"));
    }

    #[test]
    fn no_signals_for_a_clean_session() {
        let events = vec![ev("command", "handled → specs"), ev("answer", "ok")];
        assert!(detect_signals(&events).is_empty());
    }

    #[test]
    fn is_deterministic_and_ordered() {
        let events = vec![
            ev("error", "a"),
            ev("error", "b"),
            ev("command", "[exit 2]"),
            ev("command", "amendments/AMD-001.md"),
        ];
        let first = detect_signals(&events);
        let second = detect_signals(&events);
        assert_eq!(first, second);
        let kinds: Vec<&str> = first.iter().map(|s| s.kind.as_str()).collect();
        assert_eq!(kinds, vec!["repeated-errors", "command-failure", "amendment"]);
    }

    #[test]
    fn serializes_signal_in_camel_case() {
        let serialized = serde_json::to_value(Signal {
            kind: "amendment".to_string(),
            summary: "1 spec amendment(s) recorded".to_string(),
            count: 1,
        })
        .unwrap();
        assert_eq!(serialized["kind"], "amendment");
        assert_eq!(serialized["count"], 1);
    }
}
