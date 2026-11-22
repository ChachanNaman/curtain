//! Tiny sidecar CLI (PRD F5). Invoked by Claude Code / Codex hooks, not
//! by the user. Always exits 0 so a slow or dead Curtain app never
//! blocks the agent's hook.
//!
//! Usage:
//!   curtain-notify --agent claude --event needs_input
//!   curtain-notify --agent claude --event done
//!   curtain-notify --agent codex --from-codex-json '{"type":"agent-turn-complete",...}'
//!   curtain-notify --agent <id> --event waiting|done|crashed --exit-code <n>
//!   curtain-notify --agent <id> --event custom --message "..."

use app_lib::alerts::config;
use app_lib::alerts::server::PORT;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut agent: Option<String> = None;
    let mut event: Option<String> = None;
    let mut message: Option<String> = None;
    let mut exit_code: Option<i32> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--agent" => {
                agent = args.get(i + 1).cloned();
                i += 2;
            }
            "--event" => {
                event = args.get(i + 1).cloned();
                i += 2;
            }
            "--message" => {
                message = args.get(i + 1).cloned();
                i += 2;
            }
            "--exit-code" => {
                exit_code = args.get(i + 1).and_then(|s| s.parse().ok());
                i += 2;
            }
            "--from-codex-json" => {
                if let Some(json) = args.get(i + 1) {
                    event = parse_codex_event(json);
                }
                i += 2;
            }
            _ => i += 1,
        }
    }

    let (Some(agent), Some(event)) = (agent, event) else {
        std::process::exit(0);
    };

    let cfg = config::load();
    let body = serde_json::json!({
        "agent": agent,
        "event": event,
        "token": cfg.token,
        "message": message,
        "exit_code": exit_code,
    });

    let _ = ureq::post(&format!("http://127.0.0.1:{PORT}/"))
        .timeout(std::time::Duration::from_secs(2))
        .send_json(body);
}

fn parse_codex_event(json: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    match value.get("type").and_then(|t| t.as_str()) {
        Some("agent-turn-complete") => Some("done".to_string()),
        _ => None,
    }
}
