//! Local-only HTTP listener (PRD F5). Bound to 127.0.0.1 so nothing off
//! the machine can reach it; every request must carry the token from
//! `config::AlertsConfig` or it's rejected.

use serde::Deserialize;
use tauri::{AppHandle, Runtime};
use tiny_http::{Response, Server};

use super::config;
use super::fire_alert;
use crate::agents::{self, AgentStatus};

pub const PORT: u16 = 47823;

#[derive(Deserialize)]
struct EventBody {
    agent: String,
    event: String,
    token: String,
    message: Option<String>,
    exit_code: Option<i32>,
}

pub fn spawn<R: Runtime + 'static>(app: AppHandle<R>) {
    std::thread::spawn(move || {
        let addr = format!("127.0.0.1:{PORT}");
        let server = match Server::http(&addr) {
            Ok(s) => s,
            Err(e) => {
                log::error!("curtain alert listener failed to bind {addr}: {e}");
                return;
            }
        };

        for mut request in server.incoming_requests() {
            let mut body = String::new();
            let _ = request.as_reader().read_to_string(&mut body);

            let result = serde_json::from_str::<EventBody>(&body);
            let response = match result {
                Ok(evt) if evt.token == config::get().token => {
                    handle_event(&app, &evt.agent, &evt.event, evt.exit_code, evt.message.as_deref());
                    Response::from_string("ok")
                }
                Ok(_) => Response::from_string("forbidden").with_status_code(403),
                Err(_) => Response::from_string("bad request").with_status_code(400),
            };
            let _ = request.respond(response);
        }
    });
}

fn handle_event<R: Runtime>(
    app: &AppHandle<R>,
    agent: &str,
    event: &str,
    exit_code: Option<i32>,
    message: Option<&str>,
) {
    // "needs_input" is kept as an alias of "waiting" for backward
    // compatibility with the original Claude/Codex-only integration.
    let status = match event {
        "needs_input" | "waiting" => AgentStatus::Waiting,
        "done" => AgentStatus::Done,
        "crashed" => AgentStatus::Crashed,
        "custom" => {
            if let Some(msg) = message {
                super::fire_custom_alert(app, agent, msg);
            }
            return;
        }
        _ => return,
    };
    agents::set_status(agent, status, exit_code);
    fire_alert(app, agent, status, exit_code);
}
