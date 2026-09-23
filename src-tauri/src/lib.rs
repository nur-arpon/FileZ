//! Tauri shell: window, tray, commands. The real work is in engine.rs.

mod config;
mod engine;
mod fsutil;
mod packaged;
mod suggest;

use config::Config;
use engine::{Entry, Msg, Shared, State};
use serde::Serialize;
use std::sync::{mpsc, Arc, Mutex};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow};
use tauri_plugin_autostart::ManagerExt;

#[derive(Serialize)]
struct StartupInfo { packaged: bool, state: packaged::StartupState, locked: bool, note: &'static str }

#[derive(Serialize)]
struct Snapshot {
    startup: StartupInfo,
    config: Config,
    recent: Vec<Entry>,
    filed_total: usize,
    filed_today: usize,
    waiting: Vec<engine::Waiting>,
    paused: bool,
    version: String,
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") { let _ = w.show(); let _ = w.unminimize(); let _ = w.set_focus(); }
}

/// Bottom-centre of the monitor the main window is on (or the primary one), then show.
pub fn show_toast(app: &AppHandle, entries: &[Entry]) {
    let Some(t) = app.get_webview_window("toast") else { return };
    let monitor = app.get_webview_window("main").and_then(|w| w.current_monitor().ok().flatten()).or_else(|| t.primary_monitor().ok().flatten());
    if let Some(m) = monitor {
        let scale = m.scale_factor();
        let size = m.size();
        let pos = m.position();
        let (w, h) = ((560.0 * scale) as i32, (96.0 * scale) as i32);
        let _ = t.set_size(tauri::PhysicalSize::new(w as u32, h as u32));
        let x = pos.x + (size.width as i32 - w) / 2;
        let y = pos.y + size.height as i32 - h - (64.0 * scale) as i32;
        let _ = t.set_position(PhysicalPosition::new(x, y));
    }
    // one theme setting drives the main window and the toast alike
    let theme = app.state::<Shared>().lock().unwrap().config.theme.clone();
    let _ = t.emit("toast", serde_json::json!({ "entries": entries, "theme": theme }));
    let _ = t.show();
}

fn start_of_today_ms() -> i64 {
    // local midnight without pulling in chrono: use the OS's local time via the C runtime offset is
    // overkill here; "today" = last 24 h is close enough for a status card.
    fsutil::now_ms() - 86_400_000
}

#[tauri::command]
fn get_state(shared: tauri::State<Shared>, app: AppHandle) -> Snapshot {
    let mut s = shared.lock().unwrap();
    let paused = s.effective_paused();
    let since = start_of_today_ms();
    let live: Vec<&Entry> = s.history.iter().filter(|e| !e.undone).collect();
    let st = packaged::startup_task_state();
    Snapshot {
        startup: StartupInfo { packaged: packaged::is_packaged(), state: st, locked: packaged::is_packaged() && !packaged::app_may_change(st), note: if packaged::is_packaged() { packaged::locked_note(st) } else { "" } },
        config: s.config.clone(),
        recent: s.history.iter().rev().take(200).cloned().collect(),
        filed_total: live.len(),
        filed_today: live.iter().filter(|e| e.time >= since).count(),
        waiting: engine::waiting(&s),
        paused,
        version: app.package_info().version.to_string(),
    }
}

#[tauri::command]
fn save_config(shared: tauri::State<Shared>, app: AppHandle, config: Config) -> Result<(), String> {
    let mut cfg = config;
    cfg.normalise();
    let watched_changed;
    {
        let mut s = shared.lock().unwrap();
        watched_changed = s.config.watched != cfg.watched;
        s.config = cfg.clone();
        s.save_config();
        if watched_changed { let _ = s.tx.send(Msg::Reload); }
    }
    apply_autostart(&app, cfg.autostart);
    let _ = app.emit("config-changed", ());
    Ok(())
}

/// Packaged: Windows' startup task. Unpackaged: the Run key via the autostart plugin.
fn apply_autostart(app: &AppHandle, want: bool) {
    if packaged::is_packaged() {
        let st = packaged::startup_task_state();
        if packaged::app_may_change(st) && (st == packaged::StartupState::Enabled) != want { packaged::set_startup_task(want); }
    } else {
        let auto = app.autolaunch();
        let _ = if want { auto.enable() } else { auto.disable() };
    }
}

#[tauri::command]
fn count_existing(config: Config) -> usize { let mut c = config; c.normalise(); engine::count_existing(&c) }

#[tauri::command]
fn scan_existing(config: Config) -> suggest::Scan { let mut c = config; c.normalise(); suggest::scan(&c) }

#[tauri::command]
fn set_paused(shared: tauri::State<Shared>, paused: bool, minutes: Option<u64>) {
    let mut s = shared.lock().unwrap();
    s.config.paused = paused;
    s.config.paused_until = if paused { minutes.map(|m| fsutil::now_ms() + (m as i64) * 60_000) } else { None };
    s.save_config();
    if !paused { let _ = s.tx.send(Msg::TidyNow); }
}

#[tauri::command]
fn tidy_now(shared: tauri::State<Shared>) { let _ = shared.lock().unwrap().tx.send(Msg::TidyNow); }

#[tauri::command]
fn undo(shared: tauri::State<Shared>, id: i64) -> Result<Entry, String> { engine::undo(&shared, id) }

#[tauri::command]
fn undo_since(shared: tauri::State<Shared>, since: i64) -> Result<usize, String> {
    let ids: Vec<i64> = shared.lock().unwrap().history.iter().filter(|e| !e.undone && e.time >= since).map(|e| e.id).collect();
    let mut n = 0;
    for id in ids { if engine::undo(&shared, id).is_ok() { n += 1; } }
    Ok(n)
}

#[tauri::command]
fn folder_tree(shared: tauri::State<Shared>) -> Vec<engine::FolderInfo> { engine::folder_tree(&shared.lock().unwrap()) }

#[tauri::command]
fn drives() -> Vec<fsutil::Drive> { fsutil::drives() }

#[tauri::command]
fn known_folders() -> Vec<(String, String)> {
    let mut v = vec![];
    if let Some(p) = dirs::download_dir() { v.push(("Downloads".to_string(), p.to_string_lossy().to_string())); }
    if let Some(p) = dirs::desktop_dir() { v.push(("Desktop".to_string(), p.to_string_lossy().to_string())); }
    if let Some(p) = dirs::document_dir() { v.push(("Documents".to_string(), p.to_string_lossy().to_string())); }
    v
}

#[tauri::command]
fn log_frontend(shared: tauri::State<Shared>, msg: String) { shared.lock().unwrap().log(&format!("frontend  {msg}")); }

#[tauri::command]
fn hide_toast(app: AppHandle) { if let Some(t) = app.get_webview_window("toast") { let _ = t.hide(); } }

#[tauri::command]
fn preview_toast(app: AppHandle) {
    let e = Entry { id: 0, time: fsutil::now_ms(), from: "Downloads\\setup.exe".into(), to: "Downloads\\Installers\\setup.exe".into(), rule: "Installers".into(), folder: "Installers".into(), undone: false };
    show_toast(&app, &[e]);
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItemBuilder::with_id("open", "Open FileZ").build(app)?;
    let now = MenuItemBuilder::with_id("tidy_now", "Tidy now").build(app)?;
    let pause = MenuItemBuilder::with_id("pause", "Pause for 1 hour").build(app)?;
    let resume = MenuItemBuilder::with_id("resume", "Resume").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
    let menu = MenuBuilder::new(app).items(&[&open, &now, &pause, &resume]).separator().item(&quit).build()?;
    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().cloned().expect("icon"))
        .tooltip("FileZ")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, ev| {
            let shared = app.state::<Shared>();
            match ev.id().as_ref() {
                "open" => show_main(app),
                "tidy_now" => { let _ = shared.lock().unwrap().tx.send(Msg::TidyNow); }
                "pause" => { let mut s = shared.lock().unwrap(); s.config.paused = true; s.config.paused_until = Some(fsutil::now_ms() + 3_600_000); s.save_config(); let _ = app.emit("config-changed", ()); }
                "resume" => { let mut s = shared.lock().unwrap(); s.config.paused = false; s.config.paused_until = None; s.save_config(); let _ = s.tx.send(Msg::TidyNow); let _ = app.emit("config-changed", ()); }
                "quit" => { let _ = shared.lock().unwrap().tx.send(Msg::Quit); app.exit(0); }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, ev| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = ev { show_main(tray.app_handle()); }
        })
        .build(app)?;
    Ok(())
}

fn hide_on_close(w: &WebviewWindow) {
    let win = w.clone();
    w.on_window_event(move |ev| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = ev { api.prevent_close(); let _ = win.hide(); }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let hidden_start = std::env::args().any(|a| a == "--hidden");
    // State is registered on the builder, before any window exists: the webview can call
    // get_state before setup() runs, and an unmanaged state made the first screen blank.
    let data_dir = engine::data_dir();
    let _ = std::fs::create_dir_all(&data_dir);
    let (tx, rx) = mpsc::channel::<Msg>();
    let state = State::new(data_dir, tx);
    let setup_done = state.config.setup_done;
    let want_autostart = state.config.autostart;
    let shared: Shared = Arc::new(Mutex::new(state));
    let rx = Mutex::new(Some(rx));
    tauri::Builder::default()
        .manage(shared.clone())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--hidden"])))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_state, save_config, set_paused, tidy_now, undo, undo_since, folder_tree, drives, known_folders, hide_toast, preview_toast, log_frontend, count_existing, scan_existing])
        .setup(move |app| {
            let handle = app.handle().clone();
            if let Some(rx) = rx.lock().unwrap().take() { engine::start(handle.clone(), shared.clone(), rx); }
            build_tray(&handle)?;
            if let Some(w) = app.get_webview_window("main") {
                hide_on_close(&w);
                if !(hidden_start && setup_done) { let _ = w.show(); }
            }
            if setup_done { apply_autostart(&handle, want_autostart); }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building FileZ")
        // closing the last window must not quit (tray app); an explicit Quit (code set) must
        .run(|_app, ev| { if let tauri::RunEvent::ExitRequested { api, code, .. } = ev { if code.is_none() { api.prevent_exit(); } } });
}
