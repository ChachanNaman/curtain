pub mod agents;
pub mod alerts;
mod auth;
mod autostart;
mod awake;
mod cover;
mod hotkeys;
mod idle;
mod settings;

use std::time::Duration;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

const POPOVER_LABEL: &str = "popover";
const MAIN_LABEL: &str = "main";

fn toggle_popover(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window(POPOVER_LABEL) {
        let visible = win.is_visible().unwrap_or(false);
        if visible {
            let _ = win.hide();
        } else {
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
}

/// Opens the full app window (PRD §5B). Mac: switches the activation
/// policy to `Regular` so a Dock icon appears while it's open, and back
/// to `Accessory` (menu-bar-only) once it's closed.
pub(crate) fn show_app_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Regular).ok();
    if let Some(win) = app.get_webview_window(MAIN_LABEL) {
        let _ = win.show();
        let _ = win.set_focus();
    }
    if let Some(win) = app.get_webview_window(POPOVER_LABEL) {
        let _ = win.hide();
    }
}

#[tauri::command]
fn open_app_window(app: tauri::AppHandle) {
    show_app_window(&app);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new()
            .with_handler(hotkeys::handle)
            .build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            cover::cover_display,
            cover::uncover_display,
            cover::cover_all,
            cover::uncover_all,
            cover::cover_state,
            cover::get_displays,
            cover::set_dim,
            cover::dim_state,
            open_app_window,
            awake::set_keep_awake_mode,
            awake::get_keep_awake_mode,
            awake::is_keep_awake_active,
            agents::get_agents,
            alerts::get_alerts_config,
            alerts::set_alert_channel,
            alerts::send_test_alert,
            alerts::connect_claude_code,
            alerts::disconnect_claude_code,
            alerts::claude_connected,
            alerts::connect_codex,
            alerts::disconnect_codex,
            alerts::codex_connected,
            alerts::connect_gemini,
            alerts::disconnect_gemini,
            alerts::gemini_connected,
            alerts::connect_cursor,
            alerts::disconnect_cursor,
            alerts::cursor_connected,
            alerts::connect_copilot,
            alerts::disconnect_copilot,
            alerts::copilot_connected,
            alerts::connect_qwen,
            alerts::disconnect_qwen,
            alerts::qwen_connected,
            alerts::connect_aider,
            alerts::disconnect_aider,
            alerts::aider_connected,
            agents::profiles::get_agent_profiles,
            agents::profiles::upsert_agent_profile,
            agents::profiles::delete_agent_profile,
            agents::profiles::set_agent_profile_enabled,
            agents::profiles::duplicate_agent_profile,
            agents::profiles::restore_preset_agents,
            agents::list_running_processes,
            agents::test_match_rules,
            agents::history::get_agent_history,
            agents::history::get_time_saved_this_week_ms,
            auth::set_unlock_required,
            auth::is_unlock_required,
            hotkeys::get_hotkeys,
            hotkeys::set_hotkey,
            hotkeys::clear_hotkey,
            hotkeys::set_hotkey_enabled,
            hotkeys::reset_hotkey,
            hotkeys::reset_all_hotkeys,
            autostart::set_autostart,
            autostart::is_autostart_enabled,
            idle::set_idle_auto_cover,
            idle::get_idle_auto_cover,
            settings::export_settings,
            settings::import_settings,
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Popover window: hidden until the tray icon is clicked.
            let popover = WebviewWindowBuilder::new(
                app,
                POPOVER_LABEL,
                WebviewUrl::App("popover.html".into()),
            )
            .title("Curtain")
            .inner_size(360.0, 520.0)
            .resizable(false)
            .decorations(false)
            .always_on_top(true)
            .visible(false)
            .skip_taskbar(true)
            .transparent(true)
            .build()?;

            #[cfg(target_os = "macos")]
            {
                use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial, NSVisualEffectState};
                let _ = apply_vibrancy(
                    &popover,
                    NSVisualEffectMaterial::Popover,
                    Some(NSVisualEffectState::Active),
                    Some(18.0),
                );
            }
            let _ = popover;

            let show_item = MenuItem::with_id(app, "show", "Open Curtain", true, None::<&str>)?;
            let panic_item = MenuItem::with_id(app, "panic", "Cover All", true, None::<&str>)?;
            let quit_item = PredefinedMenuItem::quit(app, Some("Quit Curtain"))?;
            let menu = Menu::with_items(app, &[&show_item, &panic_item, &quit_item])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    use tauri::tray::TrayIconEvent;
                    tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
                    if let TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_popover(tray.app_handle());
                    }
                })
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_app_window(app),
                    "panic" => cover::cover_all(app.clone()),
                    _ => {}
                })
                .build(app)?;

            hotkeys::reapply(app.handle());

            alerts::start(app.handle().clone());

            // Polls running processes for agent matchers (F4) every 3s
            // and drives Timer/While-agents-run keep-awake expiry off
            // the same tick.
            let app_handle = app.handle().clone();
            std::thread::spawn(move || loop {
                let any_running = agents::poll_once(&app_handle);
                awake::tick(any_running);
                idle::tick(&app_handle);
                std::thread::sleep(Duration::from_secs(3));
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == POPOVER_LABEL
                && let tauri::WindowEvent::Focused(false) = event
            {
                let _ = window.hide();
            }
            if window.label() == MAIN_LABEL
                && let tauri::WindowEvent::CloseRequested { api, .. } = event
            {
                // Keep the app alive in the menu bar instead of quitting.
                api.prevent_close();
                let _ = window.hide();
                #[cfg(target_os = "macos")]
                window.app_handle().set_activation_policy(tauri::ActivationPolicy::Accessory).ok();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
