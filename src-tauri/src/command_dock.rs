use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedCommand {
    pub raw: String,
    pub kind: CommandKind,
    pub verb: String,
    pub argument: String,
    pub requires_approval: bool,
    pub summary: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandKind {
    Navigate,
    Ask,
    Reference,
    Tag,
    Shell,
    Steer,
    Natural,
    Empty,
}

impl CommandKind {
    fn summary(&self) -> &'static str {
        match self {
            Self::Navigate => {
                "Navigate intent recognized; navigation routing arrives in a later spec."
            }
            Self::Ask => "Ask intent recognized; artifact questions arrive in a later spec.",
            Self::Reference => {
                "Reference intent recognized; reference resolution arrives in a later spec."
            }
            Self::Tag => "Tag intent recognized; tag resolution arrives in a later spec.",
            Self::Shell => {
                "Shell intent recognized; command execution requires approval and is not yet available."
            }
            Self::Steer => "Steer intent recognized; active-turn steering arrives in a later spec.",
            Self::Natural => {
                "Natural-language intent recognized; request handling arrives in a later spec."
            }
            Self::Empty => "Empty input recognized; no action is available.",
        }
    }

    fn requires_approval(&self) -> bool {
        matches!(self, Self::Shell)
    }
}

pub fn parse_raw_command(input: &str) -> ParsedCommand {
    let trimmed_leading = input.trim_start();

    if trimmed_leading.is_empty() {
        return ParsedCommand {
            raw: input.to_string(),
            kind: CommandKind::Empty,
            verb: String::new(),
            argument: String::new(),
            requires_approval: false,
            summary: CommandKind::Empty.summary().to_string(),
        };
    }

    let first = trimmed_leading
        .chars()
        .next()
        .expect("trimmed non-empty input should have a first character");

    let (kind, verb, argument) = match first {
        '/' => prefixed_command(trimmed_leading, CommandKind::Navigate, "/"),
        '?' => prefixed_command(trimmed_leading, CommandKind::Ask, "?"),
        '@' => prefixed_command(trimmed_leading, CommandKind::Reference, "@"),
        '#' => prefixed_command(trimmed_leading, CommandKind::Tag, "#"),
        '!' => prefixed_command(trimmed_leading, CommandKind::Shell, "!"),
        '>' => prefixed_command(trimmed_leading, CommandKind::Steer, ">"),
        _ => (
            CommandKind::Natural,
            String::new(),
            trimmed_leading.trim().to_string(),
        ),
    };

    ParsedCommand {
        raw: input.to_string(),
        requires_approval: kind.requires_approval(),
        summary: kind.summary().to_string(),
        kind,
        verb,
        argument,
    }
}

fn prefixed_command(
    trimmed_leading: &str,
    kind: CommandKind,
    verb: &str,
) -> (CommandKind, String, String) {
    let argument = trimmed_leading[verb.len()..].trim().to_string();

    (kind, verb.to_string(), argument)
}

#[tauri::command]
pub fn parse_command(input: String) -> ParsedCommand {
    parse_raw_command(&input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn classifies_every_command_dock_prefix() {
        assert_command("/specs", CommandKind::Navigate, "/", "specs", false);
        assert_command(
            "? what is this",
            CommandKind::Ask,
            "?",
            "what is this",
            false,
        );
        assert_command(
            "@docs/PRD.md",
            CommandKind::Reference,
            "@",
            "docs/PRD.md",
            false,
        );
        assert_command("#FS-002", CommandKind::Tag, "#", "FS-002", false);
        assert_command("! cargo test", CommandKind::Shell, "!", "cargo test", true);
        assert_command("> pause", CommandKind::Steer, ">", "pause", false);
    }

    #[test]
    fn classifies_natural_language_without_a_verb() {
        assert_command(
            "add a workspace opener",
            CommandKind::Natural,
            "",
            "add a workspace opener",
            false,
        );
    }

    #[test]
    fn classifies_empty_and_whitespace_input() {
        assert_command("", CommandKind::Empty, "", "", false);
        assert_command("   \n\t", CommandKind::Empty, "", "", false);
    }

    #[test]
    fn ignores_leading_whitespace_when_detecting_prefix() {
        let parsed = parse_raw_command("  \t /specs  ");

        assert_eq!(parsed.kind, CommandKind::Navigate);
        assert_eq!(parsed.verb, "/");
        assert_eq!(parsed.argument, "specs");
        assert_eq!(parsed.raw, "  \t /specs  ");
    }

    #[test]
    fn keeps_prefixed_commands_prefixed_when_no_argument_is_present() {
        assert_command("?", CommandKind::Ask, "?", "", false);
        assert_command("   !   ", CommandKind::Shell, "!", "", true);
    }

    #[test]
    fn treats_grammar_glyphs_after_the_first_position_as_natural_text() {
        let parsed = parse_raw_command("email me @ 5pm");

        assert_eq!(parsed.kind, CommandKind::Natural);
        assert_eq!(parsed.verb, "");
        assert_eq!(parsed.argument, "email me @ 5pm");
        assert!(!parsed.requires_approval);
    }

    #[test]
    fn only_shell_commands_require_approval() {
        let commands = [
            parse_raw_command("/specs"),
            parse_raw_command("?"),
            parse_raw_command("@docs/PRD.md"),
            parse_raw_command("#FS-002"),
            parse_raw_command("! cargo test"),
            parse_raw_command("> stop"),
            parse_raw_command("plain text"),
            parse_raw_command(""),
        ];

        for command in commands {
            assert_eq!(
                command.requires_approval,
                command.kind == CommandKind::Shell
            );
        }
    }

    #[test]
    fn serializes_for_the_react_ipc_contract_in_camel_case() {
        let serialized = serde_json::to_value(parse_raw_command("! cargo test")).unwrap();

        assert_eq!(
            serialized,
            json!({
                "raw": "! cargo test",
                "kind": "shell",
                "verb": "!",
                "argument": "cargo test",
                "requiresApproval": true,
                "summary": "Shell intent recognized; command execution requires approval and is not yet available."
            })
        );
    }

    fn assert_command(
        input: &str,
        expected_kind: CommandKind,
        expected_verb: &str,
        expected_argument: &str,
        expected_requires_approval: bool,
    ) {
        let parsed = parse_raw_command(input);

        assert_eq!(parsed.raw, input);
        assert_eq!(parsed.kind, expected_kind);
        assert_eq!(parsed.verb, expected_verb);
        assert_eq!(parsed.argument, expected_argument);
        assert_eq!(parsed.requires_approval, expected_requires_approval);
        assert!(parsed.summary.contains("recognized") || parsed.kind == CommandKind::Empty);
    }
}
