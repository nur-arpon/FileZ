//! One look at what is actually in the watched folders, so the setup wizard can say
//! "24 PDFs, 12 SolidWorks parts, 9 files from moodle" instead of showing a fixed list.
//! Pure read-only: lists the top level of each watched folder, nothing moves.

use crate::config::{is_partial, Config, Rule};
use crate::engine::match_rule;
use crate::fsutil;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;

/// How many files a shipped rule would claim, with two example names.
#[derive(Serialize, Debug)]
pub struct RuleHit { pub id: String, pub count: usize, pub examples: Vec<String> }

/// A category FileZ made up from files no shipped rule claims. Accepting it turns it
/// into an ordinary custom rule card.
#[derive(Serialize, Debug, Clone)]
pub struct Suggestion {
    pub id: String,
    pub name: String,
    pub folder: String,
    pub extensions: Vec<String>,
    pub keywords: Vec<String>,
    pub hosts: Vec<String>,
    pub count: usize,
    pub examples: Vec<String>,
    /// "type" | "site" | "word"
    pub kind: &'static str,
}

#[derive(Serialize, Debug, Default)]
pub struct Scan {
    pub total: usize,
    pub hits: Vec<RuleHit>,
    pub suggestions: Vec<Suggestion>,
    /// extension -> count, for files nothing claims and no suggestion covers
    pub leftovers: Vec<(String, usize)>,
}

const MIN_FILES: usize = 3;

/// Extensions the shipped cards do not cover, grouped by what people call them.
/// (name, folder, extensions)
const GROUPS: &[(&str, &str, &[&str])] = &[
    ("AutoCAD drawings", "AutoCAD", &["dwg", "dxf", "dwt"]),
    ("SolidWorks files", "SolidWorks", &["slddrw", "sldprt", "sldasm"]),
    ("3D printing", "3D printing", &["gcode", "3mf", "bgcode"]),
    ("Android apps", "Android apps", &["apk", "xapk", "apks"]),
    ("Notebooks", "Notebooks", &["ipynb"]),
    ("Design files", "Design files", &["psd", "ai", "xd", "fig", "sketch", "afdesign", "afphoto", "cdr", "indd"]),
    ("Ebooks", "Ebooks", &["mobi", "azw", "azw3", "cbz", "cbr"]),
    ("Databases", "Databases", &["db", "sqlite", "sqlite3", "sql", "mdb", "accdb"]),
    ("Certificates and keys", "Certificates", &["crt", "cer", "pem", "pfx", "p12", "ppk"]),
    ("Calendar and contacts", "Calendar", &["ics", "vcf"]),
    ("Save games", "Save games", &["sav", "save"]),
    ("Minecraft", "Minecraft", &["jar", "mcpack", "mcworld", "mcaddon"]),
    ("Virtual machines", "Virtual machines", &["ova", "ovf", "vmdk", "vdi", "qcow2"]),
    ("Data files", "Data files", &["xml", "parquet", "tsv", "ndjson", "jsonl"]),
    ("Web pages", "Web pages", &["htm", "mhtml", "webarchive"]),
    ("Email", "Email", &["eml", "msg", "pst"]),
    ("Scans", "Scans", &["xps", "oxps", "djvu"]),
    ("MATLAB", "MATLAB", &["m", "mat", "slx", "mlx"]),
    ("Arduino and firmware", "Firmware", &["ino", "hex", "bin", "uf2"]),
    ("Fusion and CAD", "CAD", &["f3d", "f3z", "ipt", "iam", "prt", "catpart", "x_t", "iges", "igs"]),
];

/// Words in file names that say nothing about a category.
const STOP_WORDS: &[&str] = &[
    "final", "copy", "download", "downloads", "file", "files", "image", "photo", "video", "document", "untitled",
    "version", "draft", "new", "the", "and", "with", "from", "for", "your", "this", "that", "report", "week",
    "windows", "setup", "install", "installer", "x64", "win64", "amd64", "latest", "update",
];

pub fn scan(cfg: &Config) -> Scan {
    // Match against every shipped card as if it were on, so the wizard can show counts
    // for cards the user has not ticked yet. Custom cards keep their real state.
    let rules: Vec<Rule> = cfg.rules.iter().map(|r| { let mut r = r.clone(); if r.builtin { r.enabled = true; } r }).collect();
    let mut hits: HashMap<String, RuleHit> = HashMap::new();
    let mut unmatched: Vec<(PathBuf, String, String)> = vec![]; // path, lower name, ext
    let mut total = 0;
    for folder in &cfg.watched {
        let Ok(rd) = std::fs::read_dir(folder) else { continue };
        for e in rd.flatten() {
            let Ok(meta) = e.metadata() else { continue };
            if !meta.is_file() { continue; }
            let name = e.file_name().to_string_lossy().to_string();
            if is_partial(&name) || name.starts_with('.') { continue; }
            total += 1;
            let p = e.path();
            match match_rule(&rules, &p) {
                Some(r) => {
                    let h = hits.entry(r.id.clone()).or_insert_with(|| RuleHit { id: r.id.clone(), count: 0, examples: vec![] });
                    h.count += 1;
                    if h.examples.len() < 2 { h.examples.push(name.clone()); }
                }
                None => {
                    let ext = p.extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
                    unmatched.push((p, name.to_lowercase(), ext));
                }
            }
        }
    }
    let mut hits: Vec<RuleHit> = hits.into_values().collect();
    hits.sort_by(|a, b| b.count.cmp(&a.count));

    let mut suggestions = vec![];
    let mut taken = vec![false; unmatched.len()];

    // 1. by type: known groups first, then any single extension that repeats
    for (name, folder, exts) in GROUPS {
        let idx: Vec<usize> = (0..unmatched.len()).filter(|&i| !taken[i] && exts.contains(&unmatched[i].2.as_str())).collect();
        if idx.len() >= MIN_FILES {
            let present: Vec<String> = exts.iter().filter(|e| idx.iter().any(|&i| unmatched[i].2 == **e)).map(|e| e.to_string()).collect();
            suggestions.push(make("type", &format!("sug-type-{}", folder), name, folder, present, vec![], vec![], &idx, &unmatched));
            for i in idx { taken[i] = true; }
        }
    }
    let mut by_ext: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, u) in unmatched.iter().enumerate() { if !taken[i] && !u.2.is_empty() && u.2.len() <= 6 { by_ext.entry(u.2.clone()).or_default().push(i); } }
    let mut ext_groups: Vec<(String, Vec<usize>)> = by_ext.into_iter().filter(|(_, v)| v.len() >= MIN_FILES).collect();
    ext_groups.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
    for (ext, idx) in ext_groups {
        let label = format!("{} files", ext.to_uppercase());
        suggestions.push(make("type", &format!("sug-type-{}", ext), &label, &label, vec![ext.clone()], vec![], vec![], &idx, &unmatched));
        for i in idx { taken[i] = true; }
    }

    // 2. by website it came from
    let mut by_host: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, u) in unmatched.iter().enumerate() {
        if taken[i] { continue; }
        if let Some(h) = fsutil::source_host(&u.0) { by_host.entry(h.trim_start_matches("www.").to_string()).or_default().push(i); }
    }
    let mut host_groups: Vec<(String, Vec<usize>)> = by_host.into_iter().filter(|(_, v)| v.len() >= MIN_FILES).collect();
    host_groups.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
    for (host, idx) in host_groups {
        let folder = site_label(&host);
        suggestions.push(make("site", &format!("sug-site-{}", host), &format!("Files from {}", host), &folder, vec![], vec![], vec![host.clone()], &idx, &unmatched));
        for i in idx { taken[i] = true; }
    }

    // 3. by a word that repeats in the names
    let mut by_word: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, u) in unmatched.iter().enumerate() {
        if taken[i] { continue; }
        let stem = u.1.rsplit_once('.').map(|(s, _)| s).unwrap_or(&u.1);
        let mut seen = vec![];
        for w in stem.split(|c: char| !c.is_alphabetic()) {
            if w.len() < 5 || STOP_WORDS.contains(&w) || seen.contains(&w) { continue; }
            seen.push(w);
            by_word.entry(w.to_string()).or_default().push(i);
        }
    }
    let mut word_groups: Vec<(String, Vec<usize>)> = by_word.into_iter().filter(|(_, v)| v.len() >= MIN_FILES).collect();
    word_groups.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
    for (word, idx) in word_groups {
        let idx: Vec<usize> = idx.into_iter().filter(|&i| !taken[i]).collect();
        if idx.len() < MIN_FILES { continue; }
        let folder = capitalise(&word);
        suggestions.push(make("word", &format!("sug-word-{}", word), &format!("Files named \u{201c}{}\u{201d}", word), &folder, vec![], vec![word.clone()], vec![], &idx, &unmatched));
        for i in idx { taken[i] = true; }
    }
    suggestions.truncate(8);

    let mut left: HashMap<String, usize> = HashMap::new();
    for (i, u) in unmatched.iter().enumerate() { if !taken[i] { *left.entry(if u.2.is_empty() { "(no type)".into() } else { u.2.clone() }).or_default() += 1; } }
    let mut leftovers: Vec<(String, usize)> = left.into_iter().collect();
    leftovers.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    Scan { total, hits, suggestions, leftovers }
}

fn make(kind: &'static str, id: &str, name: &str, folder: &str, extensions: Vec<String>, keywords: Vec<String>, hosts: Vec<String>, idx: &[usize], files: &[(PathBuf, String, String)]) -> Suggestion {
    let examples = idx.iter().take(2).map(|&i| files[i].0.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()).collect();
    Suggestion { id: id.into(), name: name.into(), folder: folder.into(), extensions, keywords, hosts, count: idx.len(), examples, kind }
}

/// "moodle.monash.edu" -> "Moodle", "drive.google.com" -> "Google drive", "github.com" -> "Github"
fn site_label(host: &str) -> String {
    let parts: Vec<&str> = host.split('.').collect();
    let generic = ["com", "net", "org", "edu", "au", "uk", "io", "co", "gov", "de", "in"];
    let names: Vec<&str> = parts.iter().copied().filter(|p| !generic.contains(p) && p.len() > 1).collect();
    match names.as_slice() {
        [] => capitalise(host),
        [one] => capitalise(one),
        [sub, main, ..] => format!("{} {}", capitalise(main), sub),
    }
}

fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    match c.next() { Some(f) => f.to_uppercase().collect::<String>() + c.as_str(), None => String::new() }
}
