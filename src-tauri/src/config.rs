//! Settings and the rule model. Everything the user can change lives here and is
//! saved as one JSON file in the app's config folder.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// One rule card. Matching is first-match-wins in list order. A rule matches a file when
/// ANY of its non-empty lists hits: the download's source website, a word in the file
/// name, or the file extension.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    /// lower-case, no dot
    pub extensions: Vec<String>,
    /// lower-case substrings of the file name
    pub keywords: Vec<String>,
    /// lower-case substrings of the download's source host, e.g. "moodle"
    pub hosts: Vec<String>,
    /// folder name under `dest_root`
    pub folder: String,
    /// how long a file must sit untouched before it is moved
    pub wait_secs: u64,
    /// shipped with the app (cannot be deleted, only turned off)
    pub builtin: bool,
}

impl Default for Rule {
    fn default() -> Self {
        Rule { id: String::new(), name: String::new(), enabled: true, extensions: vec![], keywords: vec![], hosts: vec![], folder: String::new(), wait_secs: 120, builtin: false }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct Config {
    pub setup_done: bool,
    pub watched: Vec<PathBuf>,
    pub dest_root: PathBuf,
    pub rules: Vec<Rule>,
    /// 0 = never remove empty folders
    pub cleanup_empty_days: u32,
    pub autostart: bool,
    pub notifications: bool,
    pub sound: bool,
    /// "light" | "dark" | "system"
    pub theme: String,
    pub paused: bool,
    /// epoch ms; when set, `paused` clears itself at that time
    pub paused_until: Option<i64>,
    /// "rules" only for now; "gemini" | "local" | "builtin" reserved
    pub ai_mode: String,
    /// epoch ms; files last written before this are left alone ("only new files from now on")
    pub only_new_since: Option<i64>,
}

impl Default for Config {
    fn default() -> Self {
        let downloads = dirs::download_dir().unwrap_or_else(|| PathBuf::from("C:\\Downloads"));
        Config {
            setup_done: false,
            watched: vec![downloads.clone()],
            dest_root: downloads,
            rules: default_rules(),
            cleanup_empty_days: 7,
            autostart: true,
            notifications: true,
            sound: false,
            theme: "system".into(),
            paused: false,
            paused_until: None,
            ai_mode: "rules".into(),
            only_new_since: None,
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Config {
        let mut cfg: Config = std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        cfg.normalise();
        cfg
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(p) = path.parent() { std::fs::create_dir_all(p)?; }
        std::fs::write(path, serde_json::to_string_pretty(self).unwrap_or_default())
    }

    /// Lower-case the match lists, drop empties, make sure every builtin rule exists
    /// (so an app update can ship new rule cards without wiping the user's choices).
    pub fn normalise(&mut self) {
        for r in &mut self.rules {
            r.extensions = clean(&r.extensions, true);
            r.keywords = clean(&r.keywords, false);
            r.hosts = clean(&r.hosts, false);
            r.folder = r.folder.trim().trim_matches(|c| c == '\\' || c == '/').to_string();
            if r.folder.is_empty() { r.folder = r.name.clone(); }
        }
        self.rules.retain(|r| !r.folder.is_empty() && !r.id.is_empty());
        for d in default_rules() {
            if !self.rules.iter().any(|r| r.id == d.id) { self.rules.push(d); }
        }
        if self.watched.is_empty() { self.watched = Config::default().watched; }
        if !["light", "dark", "system"].contains(&self.theme.as_str()) { self.theme = "system".into(); }
    }
}

fn clean(v: &[String], strip_dot: bool) -> Vec<String> {
    let mut out: Vec<String> = v
        .iter()
        .map(|s| { let s = s.trim().to_lowercase(); if strip_dot { s.trim_start_matches('.').to_string() } else { s } })
        .filter(|s| !s.is_empty())
        .collect();
    out.dedup();
    out
}

fn rule(id: &str, name: &str, ext: &str, kw: &str, hosts: &str, folder: &str, wait_secs: u64, enabled: bool) -> Rule {
    let split = |s: &str| s.split_whitespace().map(|x| x.to_string()).collect::<Vec<_>>();
    Rule { id: id.into(), name: name.into(), enabled, extensions: split(ext), keywords: split(kw), hosts: split(hosts), folder: folder.into(), wait_secs, builtin: true }
}

/// The cards every install ships with. Order = priority: the university card sits above
/// PDFs/Documents so a lecture PDF from Moodle goes to University, not PDFs.
pub fn default_rules() -> Vec<Rule> {
    vec![
        rule("university", "University files", "", "lecture tutorial assignment syllabus", "moodle canvas blackboard brightspace .edu.", "University", 120, false),
        rule("games", "Game files", "", "", "steampowered steamcontent epicgames gog.com riotgames ubisoft ea.com battle.net", "Game files", 300, true),
        rule("installers", "Installers", "exe msi msix msixbundle appx appxbundle appinstaller", "", "", "Installers", 300, true),
        rule("zips", "Zip files", "zip 7z rar tar gz tgz xz bz2", "", "", "Zip files", 300, true),
        rule("disc", "Disc images", "iso img vhd vhdx wim", "", "", "Disc images", 300, false),
        rule("pictures", "Pictures", "png jpg jpeg gif webp bmp heic heif avif svg tif tiff", "", "", "Pictures", 120, true),
        rule("videos", "Videos", "mp4 mkv mov avi webm m4v wmv", "", "", "Videos", 120, true),
        rule("music", "Music", "mp3 wav flac m4a ogg aac opus", "", "", "Music", 120, true),
        rule("pdfs", "PDFs", "pdf", "", "", "PDFs", 120, true),
        rule("documents", "Documents", "doc docx txt rtf odt md pptx ppt odp epub", "", "", "Documents", 120, true),
        rule("spreadsheets", "Spreadsheets", "xls xlsx csv ods", "", "", "Spreadsheets", 120, true),
        rule("code", "Code", "py js ts rs json yaml yml toml html css sh ps1 c cpp h java kt go", "", "", "Code", 120, false),
        rule("fonts", "Fonts", "ttf otf woff woff2", "", "", "Fonts", 120, false),
        rule("models3d", "3D models", "stl obj step stp 3mf fbx blend sldprt sldasm", "", "", "3D models", 120, false),
        rule("torrents", "Torrents", "torrent", "", "", "Torrents", 120, false),
        rule("subtitles", "Subtitles", "srt ass vtt", "", "", "Subtitles", 120, false),
    ]
}

/// Files still being written by a browser or downloader. Never touched.
pub fn is_partial(name: &str) -> bool {
    let n = name.to_lowercase();
    n.ends_with(".crdownload") || n.ends_with(".part") || n.ends_with(".partial") || n.ends_with(".tmp") || n.ends_with(".download") || n.ends_with(".!qb") || n.ends_with(".opdownload") || n.starts_with("~$") || n == "desktop.ini" || n == "thumbs.db"
}
