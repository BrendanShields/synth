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
#[serde(rename_all = "camelCase")]
pub struct CommandRoute {
    pub parsed: ParsedCommand,
    pub disposition: RouteDisposition,
    pub target: RouteTarget,
    pub message: String,
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RouteDisposition {
    Handled,
    Unsupported,
    Blocked,
    Empty,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RouteTarget {
    Summary,
    Specs,
    RuntimeStatus,
    EventStream,
    Phase,
    None,
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

pub fn route_raw_command(input: &str) -> CommandRoute {
    let parsed = parse_raw_command(input);

    match parsed.kind {
        CommandKind::Empty => route(
            parsed,
            RouteDisposition::Empty,
            RouteTarget::None,
            "Empty input recognized; no route was evaluated.",
        ),
        CommandKind::Shell => route(
            parsed,
            RouteDisposition::Blocked,
            RouteTarget::None,
            "Shell route blocked; command execution requires approval and is not yet available.",
        ),
        CommandKind::Navigate => route_navigation(parsed),
        CommandKind::Ask => unsupported_route(
            parsed,
            "Ask routes are not available yet; artifact questions arrive in a later spec.",
        ),
        CommandKind::Reference => unsupported_route(
            parsed,
            "Reference routes are not available yet; reference resolution arrives in a later spec.",
        ),
        CommandKind::Tag => unsupported_route(
            parsed,
            "Tag routes are not available yet; tag resolution arrives in a later spec.",
        ),
        CommandKind::Steer => unsupported_route(
            parsed,
            "Steer routes are not available yet; active-turn steering arrives in a later spec.",
        ),
        CommandKind::Natural => unsupported_route(
            parsed,
            "Natural-language routes are not available yet; request handling arrives in a later spec.",
        ),
    }
}

fn route_navigation(parsed: ParsedCommand) -> CommandRoute {
    let normalized_argument = parsed.argument.to_ascii_lowercase();

    match normalized_argument.as_str() {
        "summary" => route(
            parsed,
            RouteDisposition::Handled,
            RouteTarget::Summary,
            "Handled slash navigation route to the summary section.",
        ),
        "specs" => route(
            parsed,
            RouteDisposition::Handled,
            RouteTarget::Specs,
            "Handled slash navigation route to the specs index.",
        ),
        "runtime-status" | "runtime" => route(
            parsed,
            RouteDisposition::Handled,
            RouteTarget::RuntimeStatus,
            "Handled slash navigation route to the runtime-status section.",
        ),
        "event-stream" | "events" => route(
            parsed,
            RouteDisposition::Handled,
            RouteTarget::EventStream,
            "Handled slash navigation route to the event-stream section.",
        ),
        "phase" => route(
            parsed,
            RouteDisposition::Handled,
            RouteTarget::Phase,
            "Handled slash navigation route to the phase section.",
        ),
        _ => unsupported_route(parsed, "Slash navigation route is not available yet."),
    }
}

fn unsupported_route(parsed: ParsedCommand, message: &str) -> CommandRoute {
    route(
        parsed,
        RouteDisposition::Unsupported,
        RouteTarget::None,
        message,
    )
}

fn route(
    parsed: ParsedCommand,
    disposition: RouteDisposition,
    target: RouteTarget,
    message: &str,
) -> CommandRoute {
    CommandRoute {
        parsed,
        disposition,
        target,
        message: message.to_string(),
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

#[tauri::command]
pub fn route_command(input: String) -> CommandRoute {
    route_raw_command(&input)
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

    #[test]
    fn routes_supported_slash_navigation_commands_and_aliases() {
        assert_route(
            "/summary",
            RouteDisposition::Handled,
            RouteTarget::Summary,
            "summary",
        );
        assert_route(
            "/specs",
            RouteDisposition::Handled,
            RouteTarget::Specs,
            "specs",
        );
        assert_route(
            "/runtime-status",
            RouteDisposition::Handled,
            RouteTarget::RuntimeStatus,
            "runtime-status",
        );
        assert_route(
            "/runtime",
            RouteDisposition::Handled,
            RouteTarget::RuntimeStatus,
            "runtime",
        );
        assert_route(
            "/event-stream",
            RouteDisposition::Handled,
            RouteTarget::EventStream,
            "event-stream",
        );
        assert_route(
            "/events",
            RouteDisposition::Handled,
            RouteTarget::EventStream,
            "events",
        );
        assert_route(
            "/phase",
            RouteDisposition::Handled,
            RouteTarget::Phase,
            "phase",
        );
    }

    #[test]
    fn routes_navigation_case_insensitively_after_parsing() {
        assert_route(
            "  /Runtime  ",
            RouteDisposition::Handled,
            RouteTarget::RuntimeStatus,
            "Runtime",
        );
    }

    #[test]
    fn returns_unsupported_for_unknown_slash_navigation() {
        let route = route_raw_command("/sessions");

        assert_eq!(route.parsed.kind, CommandKind::Navigate);
        assert_eq!(route.parsed.argument, "sessions");
        assert_eq!(route.disposition, RouteDisposition::Unsupported);
        assert_eq!(route.target, RouteTarget::None);
        assert!(route.message.contains("not available yet"));
    }

    #[test]
    fn returns_unsupported_for_all_non_navigation_non_shell_kinds() {
        let cases = [
            ("? what does this mean", CommandKind::Ask),
            ("@docs/PRD.md", CommandKind::Reference),
            ("#FS-003", CommandKind::Tag),
            ("> stop", CommandKind::Steer),
            ("add a workspace opener", CommandKind::Natural),
        ];

        for (input, expected_kind) in cases {
            let route = route_raw_command(input);

            assert_eq!(route.parsed.kind, expected_kind);
            assert_eq!(route.disposition, RouteDisposition::Unsupported);
            assert_eq!(route.target, RouteTarget::None);
            assert!(
                route.message.contains("later spec") || route.message.contains("not available yet")
            );
        }
    }

    #[test]
    fn blocks_shell_routes_without_execution() {
        let route = route_raw_command("! cargo test");

        assert_eq!(route.parsed.kind, CommandKind::Shell);
        assert!(route.parsed.requires_approval);
        assert_eq!(route.disposition, RouteDisposition::Blocked);
        assert_eq!(route.target, RouteTarget::None);
        assert!(route.message.contains("requires approval"));
        assert!(route.message.contains("not yet available"));
    }

    #[test]
    fn returns_empty_for_direct_empty_routing() {
        let route = route_raw_command("  \n\t");

        assert_eq!(route.parsed.kind, CommandKind::Empty);
        assert_eq!(route.disposition, RouteDisposition::Empty);
        assert_eq!(route.target, RouteTarget::None);
    }

    #[test]
    fn serializes_routes_for_the_react_ipc_contract_in_camel_case() {
        let serialized = serde_json::to_value(route_raw_command("/runtime")).unwrap();

        assert_eq!(
            serialized,
            json!({
                "parsed": {
                    "raw": "/runtime",
                    "kind": "navigate",
                    "verb": "/",
                    "argument": "runtime",
                    "requiresApproval": false,
                    "summary": "Navigate intent recognized; navigation routing arrives in a later spec."
                },
                "disposition": "handled",
                "target": "runtime-status",
                "message": "Handled slash navigation route to the runtime-status section."
            })
        );
    }

    #[test]
    fn serializes_specs_route_target_for_the_react_ipc_contract() {
        let serialized = serde_json::to_value(route_raw_command("/specs")).unwrap();

        assert_eq!(
            serialized,
            json!({
                "parsed": {
                    "raw": "/specs",
                    "kind": "navigate",
                    "verb": "/",
                    "argument": "specs",
                    "requiresApproval": false,
                    "summary": "Navigate intent recognized; navigation routing arrives in a later spec."
                },
                "disposition": "handled",
                "target": "specs",
                "message": "Handled slash navigation route to the specs index."
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

    fn assert_route(
        input: &str,
        expected_disposition: RouteDisposition,
        expected_target: RouteTarget,
        expected_argument: &str,
    ) {
        let route = route_raw_command(input);

        assert_eq!(route.parsed.kind, CommandKind::Navigate);
        assert_eq!(route.parsed.argument, expected_argument);
        assert_eq!(route.disposition, expected_disposition);
        assert_eq!(route.target, expected_target);
    }
}
