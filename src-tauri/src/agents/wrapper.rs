//! `curtain run --agent <id> -- <command...>` (PRD F4/F5 "wrapper"
//! alert source). Runs the command in a PTY so the wrapped program sees
//! a real terminal (colors, interactive prompts, line editing all work
//! normally), passes stdin/stdout straight through, and watches output
//! for the quiet-plus-prompt-pattern heuristic that signals "waiting
//! for input." Reports done/crashed (with exit code) on exit. POSTs to
//! the same local listener `curtain-notify` uses — this runs as a
//! separate process invocation with no access to the GUI app's
//! in-memory state or AppHandle.

use crate::alerts::config;
use crate::alerts::server::PORT;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use regex::Regex;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const QUIET_SECS: u64 = 20;
const PROMPT_PATTERNS: &[&str] = &[
    r"\?\s*$",
    r"(?i)\(y/n\)",
    r"(?i)\[y/n\]",
    r"(?i)press enter",
    r">\s*$",
    r"(?i)allow",
    r"(?i)approve",
];

fn post_event(agent: &str, event: &str, exit_code: Option<i32>) {
    let cfg = config::load();
    let body = serde_json::json!({
        "agent": agent,
        "event": event,
        "token": cfg.token,
        "message": serde_json::Value::Null,
        "exit_code": exit_code,
    });
    let _ = ureq::post(&format!("http://127.0.0.1:{PORT}/"))
        .timeout(Duration::from_secs(2))
        .send_json(body);
}

/// Returns the process exit code, so `main` can propagate it —
/// `curtain run` should behave like a transparent wrapper, including
/// for shell `&&`/`$?` chaining.
pub fn run(args: &[String]) -> i32 {
    let mut agent: Option<String> = None;
    let mut cmd_start: Option<usize> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--agent" => {
                agent = args.get(i + 1).cloned();
                i += 2;
            }
            "--" => {
                cmd_start = Some(i + 1);
                break;
            }
            _ => i += 1,
        }
    }

    // `--agent` is optional: a bare `curtain run -- <command>` is still
    // useful (e.g. CI one-offs) and shouldn't require registering a
    // profile first — it just reports under a generic "task" key.
    let agent = agent.unwrap_or_else(|| "task".to_string());
    let Some(start) = cmd_start else {
        eprintln!("curtain run: expected `-- <command...>`");
        return 1;
    };
    let command_args = &args[start..];
    let Some((program, rest)) = command_args.split_first() else {
        eprintln!("curtain run: no command given after `--`");
        return 1;
    };

    let pty_system = native_pty_system();
    let size = terminal_size::terminal_size()
        .map(|(w, h)| PtySize {
            rows: h.0,
            cols: w.0,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap_or(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        });

    let pair = match pty_system.openpty(size) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("curtain run: failed to open a pty: {e}");
            return 1;
        }
    };

    let mut cmd = CommandBuilder::new(program);
    cmd.args(rest);
    if let Ok(cwd) = std::env::current_dir() {
        cmd.cwd(cwd);
    }

    let mut child = match pair.slave.spawn_command(cmd) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("curtain run: failed to spawn `{program}`: {e}");
            return 1;
        }
    };
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader().expect("clone pty reader");
    let mut writer = pair.master.take_writer().expect("take pty writer");

    // stdin -> pty, on its own thread since reads block.
    std::thread::spawn(move || {
        let mut stdin = std::io::stdin();
        let mut buf = [0u8; 1024];
        loop {
            match stdin.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if writer.write_all(&buf[..n]).is_err() {
                        break;
                    }
                }
            }
        }
    });

    let last_output = Arc::new(Mutex::new(Instant::now()));
    let last_line = Arc::new(Mutex::new(String::new()));
    let running = Arc::new(AtomicBool::new(true));
    let patterns: Vec<Regex> = PROMPT_PATTERNS.iter().filter_map(|p| Regex::new(p).ok()).collect();

    // Quiet-plus-prompt-pattern watcher.
    {
        let last_output = last_output.clone();
        let last_line = last_line.clone();
        let running = running.clone();
        let agent = agent.clone();
        std::thread::spawn(move || {
            let mut already_fired = false;
            while running.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_secs(2));
                let quiet_long_enough = last_output.lock().unwrap().elapsed() >= Duration::from_secs(QUIET_SECS);
                let line = last_line.lock().unwrap().clone();
                let looks_like_prompt = patterns.iter().any(|r| r.is_match(&line));
                if quiet_long_enough && looks_like_prompt {
                    if !already_fired {
                        post_event(&agent, "waiting", None);
                        already_fired = true;
                    }
                } else {
                    already_fired = false;
                }
            }
        });
    }

    // pty -> stdout, tracking the trailing line for the prompt check.
    {
        let mut stdout = std::io::stdout();
        let mut buf = [0u8; 4096];
        let mut current_line: Vec<u8> = Vec::new();
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let _ = stdout.write_all(&buf[..n]);
                    let _ = stdout.flush();
                    *last_output.lock().unwrap() = Instant::now();
                    for &b in &buf[..n] {
                        if b == b'\n' {
                            current_line.clear();
                        } else {
                            current_line.push(b);
                        }
                    }
                    if let Ok(s) = std::str::from_utf8(&current_line) {
                        *last_line.lock().unwrap() = s.to_string();
                    }
                }
            }
        }
    }

    running.store(false, Ordering::SeqCst);
    let code = match child.wait() {
        Ok(status) => {
            if status.success() {
                0
            } else {
                status.exit_code() as i32
            }
        }
        Err(_) => 1,
    };

    post_event(&agent, if code == 0 { "done" } else { "crashed" }, Some(code));
    code
}
