use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestClassification {
    pub kind: String,
    pub spec_required: bool,
    pub baseline_required: bool,
    pub rationale: String,
}

fn looks_project(text: &str) -> bool {
    const SIGNALS: &[&str] = &[
        "product",
        "architecture",
        "authentication",
        " auth",
        "subsystem",
        "platform",
        "redesign",
        "workflow engine",
        "billing",
        "subscription",
        "payment",
        "multi-tenant",
        "new system",
        "whole app",
        "entire app",
    ];
    SIGNALS.iter().any(|signal| text.contains(signal))
        || text.starts_with("build a")
        || text.starts_with("build an")
        || text.starts_with("introduce a")
}

fn looks_change(text: &str) -> bool {
    const VERBS: &[&str] = &[
        "refactor",
        "fix",
        "rename",
        "remove",
        "implement",
        "modify",
        "tweak",
        "adjust",
        "update ",
        "add ",
        "change ",
        "create ",
    ];
    VERBS.iter().any(|verb| text.contains(verb))
}

fn looks_question(text: &str) -> bool {
    const STARTS: &[&str] = &[
        "how", "what", "why", "explain", "does", "can ", "could", "where", "which", "who",
        "when",
    ];
    text.ends_with('?') || STARTS.iter().any(|start| text.starts_with(start))
}

pub fn classify_request_text(input: &str) -> RequestClassification {
    let text = input.trim().to_lowercase();

    if text.is_empty() {
        return RequestClassification {
            kind: "question".to_string(),
            spec_required: false,
            baseline_required: false,
            rationale: "Nothing to classify.".to_string(),
        };
    }

    if looks_project(&text) {
        return RequestClassification {
            kind: "project".to_string(),
            spec_required: true,
            baseline_required: true,
            rationale: "Reads as project-level work; the planning baseline (PRD + ERD) is required before implementation.".to_string(),
        };
    }

    if looks_change(&text) {
        return RequestClassification {
            kind: "component".to_string(),
            spec_required: true,
            baseline_required: false,
            rationale: "Requests a change to a component; a feature spec is required before edits.".to_string(),
        };
    }

    let rationale = if looks_question(&text) {
        "Asks for explanation; no spec is required unless you ask to change code."
    } else {
        "Reads as a query rather than a change; treating it as a question."
    };
    RequestClassification {
        kind: "question".to_string(),
        spec_required: false,
        baseline_required: false,
        rationale: rationale.to_string(),
    }
}

#[tauri::command]
pub fn classify_request(input: String) -> RequestClassification {
    classify_request_text(&input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_questions() {
        let result = classify_request_text("How does routing work?");
        assert_eq!(result.kind, "question");
        assert!(!result.spec_required);
        assert!(!result.baseline_required);
    }

    #[test]
    fn classifies_component_changes() {
        let result = classify_request_text("Refactor the command dock component");
        assert_eq!(result.kind, "component");
        assert!(result.spec_required);
        assert!(!result.baseline_required);
    }

    #[test]
    fn classifies_project_work() {
        let result = classify_request_text("Build a new authentication system for the app");
        assert_eq!(result.kind, "project");
        assert!(result.spec_required);
        assert!(result.baseline_required);
    }

    #[test]
    fn project_signals_win_over_change_verbs() {
        let result = classify_request_text("add billing and subscriptions to the product");
        assert_eq!(result.kind, "project");
        assert!(result.baseline_required);
    }

    #[test]
    fn empty_input_is_a_question_with_nothing_to_classify() {
        let result = classify_request_text("   ");
        assert_eq!(result.kind, "question");
        assert!(!result.spec_required);
        assert!(result.rationale.contains("Nothing to classify"));
    }

    #[test]
    fn serializes_in_camel_case() {
        let serialized = serde_json::to_value(classify_request_text("explain the index")).unwrap();
        assert_eq!(serialized["kind"], "question");
        assert_eq!(serialized["specRequired"], false);
        assert_eq!(serialized["baselineRequired"], false);
        assert!(serialized["rationale"].is_string());
    }
}
