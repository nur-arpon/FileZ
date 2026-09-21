//! The background worker: watches the folders, runs a pass on every change (and every
//! 20 s as a safety net), moves files that match a rule, keeps the undo journal, and
//! removes long-empty category folders. Nothing here ever deletes a file.

use crate::config::{is_partial, Config, Rule};
use crate::fsutil;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Entry {
    pub id: i64,
    pub time: i64,
    pub from: String,
    pub to: String,
    pub rule: String,
    pub folder: String,
    #[serde(default)]
    pub undone: bool,
}

#[derive(Clone, Serialize, Debug, Default)]
pub struct Waiting { pub name: String, pub folder: String, pub reason: String }

pub enum Msg { Fs, Reload, TidyNow, Quit }

pub struct State {
    pub config: Config,
    pub config_path: PathBuf,
    pub data_dir: PathBuf,
    pub history: Vec<Entry>,
    /// files put back by Undo: left alone until they leave the watched folder
    pub ignored: HashSet<PathBuf>,
    /// category folder -> first time we saw it empty (epoch ms)
    pub empty_seen: HashMap<PathBuf, i64>,
    pub first_seen: HashMap<PathBuf, Instant>,
    pub last_cleanup: Instant,
    pub tx: Sender<Msg>,
}

pub type Shared = Arc<Mutex<State>>;

impl State {
    pub fn new(data_dir: PathBuf, tx: Sender<Msg>) -> State {
        let config_path = data_dir.join("config.json");
        let config = Config::load(&config_path);
        let history = read_jsonl(&data_dir.join("history.jsonl"));
        let ignored: HashSet<PathBuf> = std::fs::read_to_string(data_dir.join("ignored.json")).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        let empty_seen: HashMap<PathBuf, i64> = std::fs::read_to_string(data_dir.join("empty-seen.json")).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        State { config, config_path, data_dir, history, ignored, empty_seen, first_seen: HashMap::new(), last_cleanup: Instant::now() - Duration::from_secs(3600), tx }
    }

    pub fn save_config(&self) { let _ = self.config.save(&self.config_path); }
    fn save_ignored(&self) { let _ = std::fs::write(self.data_dir.join("ignored.json"), serde_json::to_string(&self.ignored).unwrap_or_default()); }
    fn save_empty_seen(&self) { let _ = std::fs::write(self.data_dir.join("empty-seen.json"), serde_json::to_string(&self.empty_seen).unwrap_or_default()); }
    fn rewrite_history(&self) {
        let body: String = self.history.iter().map(|e| serde_json::to_string(e).unwrap_or_default() + "\n").collect();
        let _ = std::fs::write(self.data_dir.join("history.jsonl"), body);
    }
    fn append_history(&mut self, e: Entry) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(self.data_dir.join("history.jsonl")) {
            let _ = writeln!(f, "{}", serde_json::to_string(&e).unwrap_or_default());
        }
        self.history.push(e);
        if self.history.len() > 5000 { self.history.drain(0..1000); self.rewrite_history(); }
    }

    pub fn log(&self, line: &str) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(self.data_dir.join("tidy.log")) {
            let _ = writeln!(f, "{}  {}", fsutil::now_ms(), line);
        }
    }

    /// Pause with an end time clears itself.
    pub fn effective_paused(&mut self) -> bool {
        if self.config.paused {
            if let Some(t) = self.config.paused_until {
                if fsutil::now_ms() >= t { self.config.paused = false; self.config.paused_until = None; self.save_config(); }
            }
        }
        self.config.paused
    }
}

fn read_jsonl(p: &Path) -> Vec<Entry> {
    std::fs::read_to_string(p).map(|s| s.lines().filter_map(|l| serde_json::from_str(l).ok()).collect()).unwrap_or_default()
}

/// True when the file was last written before the "only new files" cutoff.
fn is_old(meta: &std::fs::Metadata, cutoff: Option<i64>) -> bool {
    let Some(c) = cutoff else { return false };
    fsutil::latest_write_ms(meta).map_or(false, |t| t < c)
}

/// How many files in the given folders a rule set would move right now. Used by the wizard
/// to warn before the first pass ("Downloads already has 86 matching files").
pub fn count_existing(cfg: &Config) -> usize {
    let mut n = 0;
    for folder in &cfg.watched {
        let Ok(rd) = std::fs::read_dir(folder) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            let Ok(meta) = e.metadata() else { continue };
            if !meta.is_file() { continue; }
            let name = e.file_name().to_string_lossy().to_string();
            if is_partial(&name) || name.starts_with('.') { continue; }
            if let Some(r) = match_rule(&cfg.rules, &p) { if cfg.dest_root.join(&r.folder) != *folder { n += 1; } }
        }
    }
    n
}

/// Which rule (if any) claims this file. First match wins.
pub fn match_rule<'a>(rules: &'a [Rule], path: &Path) -> Option<&'a Rule> {
    let name = path.file_name()?.to_string_lossy().to_lowercase();
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let mut host: Option<Option<String>> = None; // lazily read, once
    for r in rules.iter().filter(|r| r.enabled) {
        if !r.hosts.is_empty() {
            let h = host.get_or_insert_with(|| fsutil::source_host(path));
            if let Some(h) = h { if r.hosts.iter().any(|x| h.contains(x.as_str())) { return Some(r); } }
        }
        if !r.keywords.is_empty() && r.keywords.iter().any(|k| name.contains(k.as_str())) { return Some(r); }
        if !ext.is_empty() && r.extensions.iter().any(|e| *e == ext) { return Some(r); }
    }
    None
}

/// Files in the watched folders that a rule claims but that have not moved yet, with why.
pub fn waiting(state: &State) -> Vec<Waiting> {
    let mut out = vec![];
    for folder in &state.config.watched {
        let Ok(rd) = std::fs::read_dir(folder) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            let Ok(meta) = e.metadata() else { continue };
            if !meta.is_file() { continue; }
            let name = e.file_name().to_string_lossy().to_string();
            if is_partial(&name) || state.ignored.contains(&p) || is_old(&meta, state.config.only_new_since) { continue; }
            let Some(rule) = match_rule(&state.config.rules, &p) else { continue };
            let age = fsutil::age_secs(&meta);
            let reason = if age < rule.wait_secs { format!("moves in {} s", rule.wait_secs - age) } else if !fsutil::is_free(&p) { "in use by another program".into() } else { "next pass".into() };
            out.push(Waiting { name, folder: rule.folder.clone(), reason });
        }
    }
    out
}

/// One pass over the watched folders. Returns how soon to run again: the earliest moment a
/// deferred file (too young, or still within its rule's wait) becomes due, else None.
fn run_pass(app: &AppHandle, shared: &Shared) -> Option<Duration> {
    let (cfg, ignored) = {
        let mut s = shared.lock().unwrap();
        if s.effective_paused() { return None; }
        // forget ignored files that are gone
        let before = s.ignored.len();
        s.ignored.retain(|p| p.exists());
        if s.ignored.len() != before { s.save_ignored(); }
        (s.config.clone(), s.ignored.clone())
    };
    if !cfg.setup_done { return None; }

    let mut next_due: Option<u64> = None;
    let mut moved: Vec<Entry> = vec![];
    for folder in &cfg.watched {
        let Ok(rd) = std::fs::read_dir(folder) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            let Ok(meta) = e.metadata() else { continue };
            if !meta.is_file() { continue; }
            let name = e.file_name().to_string_lossy().to_string();
            if is_partial(&name) || name.starts_with('.') || ignored.contains(&p) || is_old(&meta, cfg.only_new_since) { continue; }
            let Some(rule) = match_rule(&cfg.rules, &p) else { continue };
            let dest_dir = cfg.dest_root.join(&rule.folder);
            if dest_dir == *folder || p.starts_with(&dest_dir) { continue; }
            // must have been seen by us for a few seconds AND untouched for the rule's wait
            let seen_for = {
                let mut s = shared.lock().unwrap();
                s.first_seen.entry(p.clone()).or_insert_with(Instant::now).elapsed()
            };
            let age = fsutil::age_secs(&meta);
            if seen_for < Duration::from_secs(3) || age < rule.wait_secs {
                let wait = (3u64.saturating_sub(seen_for.as_secs())).max(rule.wait_secs.saturating_sub(age)) + 1;
                next_due = Some(next_due.map_or(wait, |d| d.min(wait)));
                continue;
            }
            if !fsutil::is_free(&p) { next_due = Some(next_due.map_or(5, |d| d.min(5))); continue; }
            let target = fsutil::unique_target(&dest_dir, &name);
            match fsutil::move_file(&p, &target) {
                Ok(()) => {
                    let entry = Entry { id: fsutil::now_ms() + moved.len() as i64, time: fsutil::now_ms(), from: p.to_string_lossy().to_string(), to: target.to_string_lossy().to_string(), rule: rule.name.clone(), folder: rule.folder.clone(), undone: false };
                    let mut s = shared.lock().unwrap();
                    s.first_seen.remove(&p);
                    s.log(&format!("moved  {}  ->  {}", entry.from, entry.to));
                    s.append_history(entry.clone());
                    moved.push(entry);
                }
                Err(err) => { shared.lock().unwrap().log(&format!("could not move {}: {err}", p.display())); }
            }
        }
    }
    // drop first_seen entries for files that vanished
    { let mut s = shared.lock().unwrap(); s.first_seen.retain(|p, _| p.exists()); }

    if !moved.is_empty() {
        let _ = app.emit("filed", &moved);
        if cfg.notifications { crate::show_toast(app, &moved); }
    }

    let due = shared.lock().unwrap().last_cleanup.elapsed() > Duration::from_secs(3600);
    if due { cleanup_empty(shared); }
    next_due.map(Duration::from_secs)
}

/// A category folder that has been empty for `cleanup_empty_days` is removed. Only
/// folders named by a rule under `dest_root`, never anything else.
fn cleanup_empty(shared: &Shared) {
    let mut s = shared.lock().unwrap();
    s.last_cleanup = Instant::now();
    let days = s.config.cleanup_empty_days;
    if days == 0 { s.empty_seen.clear(); s.save_empty_seen(); return; }
    let now = fsutil::now_ms();
    let root = s.config.dest_root.clone();
    let folders: Vec<PathBuf> = s.config.rules.iter().map(|r| root.join(&r.folder)).collect();
    let mut changed = false;
    for dir in folders {
        let empty = dir.is_dir() && std::fs::read_dir(&dir).map(|mut rd| rd.next().is_none()).unwrap_or(false);
        if !empty { if s.empty_seen.remove(&dir).is_some() { changed = true; } continue; }
        let since = *s.empty_seen.entry(dir.clone()).or_insert(now);
        changed = true;
        if now - since >= (days as i64) * 86_400_000 {
            if std::fs::remove_dir(&dir).is_ok() { s.log(&format!("removed empty folder {}", dir.display())); }
            s.empty_seen.remove(&dir);
        }
    }
    s.empty_seen.retain(|p, _| p.is_dir());
    if changed { s.save_empty_seen(); }
}

/// Put one filed item back where it came from.
pub fn undo(shared: &Shared, id: i64) -> Result<Entry, String> {
    let mut s = shared.lock().unwrap();
    let idx = s.history.iter().position(|e| e.id == id).ok_or("not found")?;
    let e = s.history[idx].clone();
    if e.undone { return Err("already put back".into()); }
    let to = PathBuf::from(&e.to);
    let from = PathBuf::from(&e.from);
    if !to.exists() { return Err("the file is no longer where Tidy put it".into()); }
    let back = if from.exists() { fsutil::unique_target(from.parent().unwrap_or(Path::new(".")), &from.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()) } else { from.clone() };
    fsutil::move_file(&to, &back).map_err(|e| e.to_string())?;
    s.history[idx].undone = true;
    s.rewrite_history();
    s.ignored.insert(back.clone());
    s.save_ignored();
    s.log(&format!("put back  {}  ->  {}", e.to, back.display()));
    Ok(s.history[idx].clone())
}

/// The worker thread. Owns the file watcher; rebuilds it when the watched list changes.
pub fn start(app: AppHandle, shared: Shared, rx: Receiver<Msg>) {
    std::thread::Builder::new().name("tidy-engine".into()).spawn(move || {
        let mut watcher: Option<notify::RecommendedWatcher> = None;
        let rebuild = |shared: &Shared| -> Option<notify::RecommendedWatcher> {
            use notify::Watcher;
            let tx = shared.lock().unwrap().tx.clone();
            let mut w = notify::recommended_watcher(move |_res: notify::Result<notify::Event>| { let _ = tx.send(Msg::Fs); }).ok()?;
            for f in &shared.lock().unwrap().config.watched {
                if let Err(e) = w.watch(f, notify::RecursiveMode::NonRecursive) { log::warn!("cannot watch {}: {e}", f.display()); }
            }
            Some(w)
        };
        watcher = rebuild(&shared).or(watcher);
        let fallback = Duration::from_secs(20);
        let mut wake = Duration::from_secs(1); // first pass right after launch; later shortened when files wait on a timer
        let mut debounce_until = Instant::now();
        loop {
            let next = match rx.recv_timeout(wake) {
                Ok(Msg::Quit) => break,
                Ok(Msg::Reload) => { watcher = rebuild(&shared); run_pass(&app, &shared) }
                Ok(Msg::TidyNow) => { shared.lock().unwrap().first_seen.clear(); run_pass_ignoring_seen(&app, &shared) }
                Ok(Msg::Fs) => {
                    // burst of events while a download is written: wait for quiet, then pass
                    if Instant::now() < debounce_until { continue; }
                    std::thread::sleep(Duration::from_millis(1500));
                    while rx.try_recv().is_ok() {}
                    debounce_until = Instant::now() + Duration::from_millis(500);
                    run_pass(&app, &shared)
                }
                Err(mpsc::RecvTimeoutError::Timeout) => run_pass(&app, &shared),
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            };
            wake = next.map_or(fallback, |d| d.clamp(Duration::from_secs(1), fallback));
        }
        drop(watcher);
    }).expect("engine thread");
}

/// "Tidy now": skip the 3-second seen check but still honour each rule's wait time
/// (a half-written download must not be moved).
fn run_pass_ignoring_seen(app: &AppHandle, shared: &Shared) -> Option<Duration> {
    {
        let mut s = shared.lock().unwrap();
        let cfg = s.config.clone();
        for folder in &cfg.watched {
            if let Ok(rd) = std::fs::read_dir(folder) {
                for e in rd.flatten() { s.first_seen.insert(e.path(), Instant::now() - Duration::from_secs(10)); }
            }
        }
    }
    run_pass(app, shared)
}

/// Folder tree under dest_root: category folders with file counts.
#[derive(Serialize, Clone, Debug)]
pub struct FolderInfo { pub name: String, pub path: String, pub files: u64, pub exists: bool, pub empty_since: Option<i64> }

pub fn folder_tree(state: &State) -> Vec<FolderInfo> {
    let mut seen = HashSet::new();
    let mut out = vec![];
    for r in &state.config.rules {
        if !seen.insert(r.folder.clone()) { continue; }
        let path = state.config.dest_root.join(&r.folder);
        let exists = path.is_dir();
        let files = if exists { std::fs::read_dir(&path).map(|rd| rd.flatten().filter(|e| e.path().is_file()).count() as u64).unwrap_or(0) } else { 0 };
        out.push(FolderInfo { name: r.folder.clone(), path: path.to_string_lossy().to_string(), files, exists, empty_since: state.empty_seen.get(&path).copied() });
    }
    out
}

/// %APPDATA%\com.arpon.tidy — the same folder Tauri's app_data_dir() resolves to on Windows,
/// computed without an AppHandle so the state can exist before the app is built.
pub fn data_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("com.arpon.tidy")
}
