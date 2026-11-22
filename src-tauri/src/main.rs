// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
  // `curtain run --agent <id> -- <command...>` is the PTY wrapper CLI
  // (PRD F4/F5): intercepted before Tauri boots so the same installed
  // binary doubles as both the GUI app and the terminal tool, with no
  // separate binary-name collision to resolve.
  let args: Vec<String> = std::env::args().skip(1).collect();
  if args.first().map(String::as_str) == Some("run") {
    let code = app_lib::agents::wrapper::run(&args[1..]);
    std::process::exit(code);
  }

  app_lib::run();
}
