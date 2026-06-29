use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub fn truncate_output(text: &str, cap: usize) -> String {
    if text.chars().count() <= cap {
        text.to_string()
    } else {
        let head: String = text.chars().take(cap).collect();
        format!("{head}\n[output truncated]")
    }
}

pub fn format_output(stdout: &str, stderr: &str, code: Option<i32>) -> String {
    let mut out = String::new();
    out.push_str(stdout);
    if !stderr.trim().is_empty() {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("[stderr]\n");
        out.push_str(stderr);
    }
    match code {
        Some(0) => {}
        Some(code) => out.push_str(&format!("\n[exit {code}]")),
        None => out.push_str("\n[terminated by signal]"),
    }
    if out.trim().is_empty() {
        "[no output]".to_string()
    } else {
        out
    }
}

pub fn run_command(
    root: &Path,
    command: &str,
    timeout_secs: u64,
    cap: usize,
) -> Result<String, String> {
    let (sender, receiver) = mpsc::channel();
    let command = command.to_string();
    let root = root.to_path_buf();

    thread::spawn(move || {
        let result = std::process::Command::new("sh")
            .arg("-c")
            .arg(&command)
            .current_dir(&root)
            .output();
        let _ = sender.send(result);
    });

    match receiver.recv_timeout(Duration::from_secs(timeout_secs)) {
        Ok(Ok(output)) => {
            let formatted = format_output(
                &String::from_utf8_lossy(&output.stdout),
                &String::from_utf8_lossy(&output.stderr),
                output.status.code(),
            );
            Ok(truncate_output(&formatted, cap))
        }
        Ok(Err(error)) => Err(format!("Could not run command: {error}")),
        Err(_) => Ok(format!("[timed out after {timeout_secs}s]")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_output_to_the_cap() {
        assert_eq!(truncate_output("abc", 10), "abc");
        assert!(truncate_output("abcdef", 3).starts_with("abc"));
        assert!(truncate_output("abcdef", 3).contains("truncated"));
    }

    #[test]
    fn formats_output_with_stderr_and_exit() {
        assert_eq!(format_output("hi\n", "", Some(0)), "hi\n");
        assert!(format_output("", "boom", Some(1)).contains("[stderr]"));
        assert!(format_output("", "boom", Some(1)).contains("[exit 1]"));
        assert_eq!(format_output("", "", Some(0)), "[no output]");
    }

    #[test]
    fn runs_a_safe_command_in_a_temp_dir() {
        let dir = std::env::temp_dir();
        let ok = run_command(&dir, "echo hello", 10, 4000).unwrap();
        assert!(ok.contains("hello"));

        let failing = run_command(&dir, "exit 3", 10, 4000).unwrap();
        assert!(failing.contains("[exit 3]"));
    }

    #[test]
    fn enforces_a_timeout() {
        let dir = std::env::temp_dir();
        let result = run_command(&dir, "sleep 5", 1, 4000).unwrap();
        assert!(result.contains("timed out"));
    }
}
