use serde::Serialize;

use crate::workspace::{KnowledgeNote, WorkspaceState};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeHit {
    pub slug: String,
    pub title: String,
    pub path: String,
    pub score: u32,
    pub snippet: String,
}

const MAX_GROUNDING_CHARS: usize = 2000;

fn query_terms(query: &str) -> Vec<String> {
    query
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|term| term.len() >= 3)
        .map(|term| term.to_string())
        .collect()
}

fn snippet_of(content: &str) -> String {
    content.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(160).collect()
}

pub fn rank_knowledge(
    docs: &[(KnowledgeNote, String)],
    query: &str,
    limit: usize,
) -> Vec<KnowledgeHit> {
    let terms = query_terms(query);
    if terms.is_empty() {
        return Vec::new();
    }

    let mut hits: Vec<KnowledgeHit> = docs
        .iter()
        .filter_map(|(note, content)| {
            let haystack = format!("{} {}", note.title, content).to_lowercase();
            let score: u32 = terms
                .iter()
                .map(|term| haystack.matches(term.as_str()).count() as u32)
                .sum();
            if score == 0 {
                return None;
            }
            Some(KnowledgeHit {
                slug: note.slug.clone(),
                title: note.title.clone(),
                path: note.path.clone(),
                score,
                snippet: snippet_of(content),
            })
        })
        .collect();

    hits.sort_by(|a, b| b.score.cmp(&a.score).then(a.slug.cmp(&b.slug)));
    hits.truncate(limit);
    hits
}

pub fn build_grounded_prompt(grounding: &[(String, String)], question: &str) -> String {
    if grounding.is_empty() {
        return question.to_string();
    }
    let context = grounding
        .iter()
        .map(|(title, content)| {
            let capped: String = content.chars().take(MAX_GROUNDING_CHARS).collect();
            format!("## {title}\n{capped}")
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    format!(
        "Use the following project knowledge to answer the question. If the \
         knowledge does not cover it, say so.\n\n{context}\n\nQuestion: {question}\n\nAnswer:"
    )
}

#[tauri::command]
pub fn retrieve_knowledge(
    workspace: tauri::State<'_, WorkspaceState>,
    query: String,
    limit: u32,
) -> Result<Vec<KnowledgeHit>, String> {
    let root = {
        let guard = workspace.0.lock().expect("workspace state lock poisoned");
        guard.as_ref().ok_or("No workspace is open.")?.root.clone()
    };
    let docs = crate::workspace::read_knowledge_in(std::path::Path::new(&root));
    Ok(rank_knowledge(&docs, &query, limit as usize))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(slug: &str, title: &str, content: &str) -> (KnowledgeNote, String) {
        (
            KnowledgeNote {
                slug: slug.to_string(),
                title: title.to_string(),
                path: format!("docs/knowledge/{slug}.md"),
            },
            content.to_string(),
        )
    }

    #[test]
    fn ranks_relevant_above_irrelevant_and_excludes_zero() {
        let docs = vec![
            doc("routing", "Routing grammar", "the routing grammar parses prefixes"),
            doc("colors", "Color palette", "warm neutral tones"),
        ];
        let hits = rank_knowledge(&docs, "routing grammar", 5);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].slug, "routing");
        assert!(hits[0].score >= 2);
    }

    #[test]
    fn respects_limit_and_is_deterministic() {
        let docs = vec![
            doc("a", "alpha routing", "routing routing"),
            doc("b", "beta routing", "routing"),
        ];
        let first = rank_knowledge(&docs, "routing", 1);
        let second = rank_knowledge(&docs, "routing", 1);
        assert_eq!(first, second);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].slug, "a");
    }

    #[test]
    fn empty_query_returns_no_hits() {
        let docs = vec![doc("a", "alpha", "routing")];
        assert!(rank_knowledge(&docs, "  a of ", 5).is_empty());
    }

    #[test]
    fn grounded_prompt_includes_context_and_question() {
        let prompt = build_grounded_prompt(
            &[("Routing".to_string(), "grammar parses prefixes".to_string())],
            "How does routing work?",
        );
        assert!(prompt.contains("Routing"));
        assert!(prompt.contains("grammar parses prefixes"));
        assert!(prompt.contains("How does routing work?"));
    }

    #[test]
    fn grounded_prompt_without_context_is_the_question() {
        assert_eq!(build_grounded_prompt(&[], "plain question"), "plain question");
    }

    #[test]
    fn serializes_hit_in_camel_case() {
        let value = serde_json::to_value(KnowledgeHit {
            slug: "routing".to_string(),
            title: "Routing".to_string(),
            path: "docs/knowledge/routing.md".to_string(),
            score: 3,
            snippet: "x".to_string(),
        })
        .unwrap();
        assert_eq!(value["slug"], "routing");
        assert_eq!(value["score"], 3);
    }
}
