//! Small file-system helpers: source URL of a download, "is anyone using this file",
//! safe move with collision rename, drive list.

use serde::Serialize;
use std::path::{Path, PathBuf};

/// Host name of the site a file was downloaded from, read from the Zone.Identifier
/// stream Windows attaches to every browser download. None on other OSes / local files.
pub fn source_host(path: &Path) -> Option<String> {
    #[cfg(windows)]
    {
        let ads = format!("{}:Zone.Identifier", path.to_string_lossy());
        let text = std::fs::read_to_string(ads).ok()?;
        for line in text.lines() {
            let line = line.trim();
            for key in ["HostUrl=", "ReferrerUrl="] {
                if let Some(url) = line.strip_prefix(key) {
                    if let Some(host) = host_of(url) { return Some(host); }
                }
            }
        }
        None
    }
    #[cfg(not(windows))]
    { let _ = path; None }
}

fn host_of(url: &str) -> Option<String> {
    let rest = url.split("://").nth(1)?;
    let host = rest.split(['/', '?', '#']).next()?.split('@').last()?.split(':').next()?;
    if host.is_empty() { None } else { Some(host.to_lowercase()) }
}

/// True when no other program holds the file open (Windows: exclusive open succeeds).
pub fn is_free(path: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        std::fs::OpenOptions::new().read(true).share_mode(0).open(path).is_ok()
    }
    #[cfg(not(windows))]
    { let _ = path; true }
}

/// `name.ext` -> `name (2).ext`, `name (3).ext`, ... until free.
pub fn unique_target(dir: &Path, name: &str) -> PathBuf {
    let first = dir.join(name);
    if !first.exists() { return first; }
    let p = Path::new(name);
    let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| name.to_string());
    let ext = p.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    for i in 2.. {
        let c = dir.join(format!("{stem} ({i}){ext}"));
        if !c.exists() { return c; }
    }
    unreachable!()
}

/// Rename; if that fails (other drive), copy then delete the original only after the
/// copy is complete and the same size.
pub fn move_file(from: &Path, to: &Path) -> std::io::Result<()> {
    if let Some(p) = to.parent() { std::fs::create_dir_all(p)?; }
    match std::fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(_) => {
            let n = std::fs::copy(from, to)?;
            let want = std::fs::metadata(from)?.len();
            if n != want {
                let _ = std::fs::remove_file(to);
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "copy size mismatch"));
            }
            std::fs::remove_file(from)
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct Drive { pub root: String, pub free: u64, pub total: u64 }

pub fn drives() -> Vec<Drive> {
    #[cfg(windows)]
    {
        use windows::core::PCWSTR;
        use windows::Win32::Storage::FileSystem::{GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives};
        const DRIVE_FIXED: u32 = 3; // winbase.h; the crate does not export the constant
        let mask = unsafe { GetLogicalDrives() };
        let mut out = vec![];
        for i in 0..26u32 {
            if mask & (1 << i) == 0 { continue; }
            let root = format!("{}:\\", (b'A' + i as u8) as char);
            let wide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
            let kind = unsafe { GetDriveTypeW(PCWSTR(wide.as_ptr())) };
            if kind != DRIVE_FIXED { continue; }
            let (mut free, mut total, mut tf) = (0u64, 0u64, 0u64);
            let ok = unsafe { GetDiskFreeSpaceExW(PCWSTR(wide.as_ptr()), Some(&mut free), Some(&mut total), Some(&mut tf)) };
            if ok.is_ok() && total > 0 { out.push(Drive { root, free, total }); }
        }
        out
    }
    #[cfg(not(windows))]
    { vec![] }
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// Epoch ms of the later of modified / created, or None if neither is readable.
pub fn latest_write_ms(meta: &std::fs::Metadata) -> Option<i64> {
    let m = meta.modified().ok();
    let c = meta.created().ok();
    let latest = match (m, c) { (Some(a), Some(b)) => Some(a.max(b)), (a, b) => a.or(b) }?;
    latest.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_millis() as i64)
}

/// Seconds since the file was last written (or created, whichever is later).
pub fn age_secs(meta: &std::fs::Metadata) -> u64 {
    let now = std::time::SystemTime::now();
    let m = meta.modified().ok();
    let c = meta.created().ok();
    let latest = match (m, c) { (Some(a), Some(b)) => Some(a.max(b)), (a, b) => a.or(b) };
    latest.and_then(|t| now.duration_since(t).ok()).map(|d| d.as_secs()).unwrap_or(u64::MAX)
}
