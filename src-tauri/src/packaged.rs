//! "Is this copy running from an MSIX (Store) package?", and the one thing that must
//! differ when it is: start-with-Windows. A packaged app's Run-key entry would point at a
//! WindowsApps folder whose name changes on every Store update, so Windows owns autostart
//! there through the manifest's windows.startupTask. Ported from Spaceadom's packaged.rs.
//!
//! CONTRACT: `STARTUP_TASK_ID` must equal desktop:StartupTask/@TaskId in msix/AppxManifest.xml.

use serde::Serialize;
use std::sync::OnceLock;

pub const STARTUP_TASK_ID: &str = "TidyUpStartupTask";

static PACKAGED: OnceLock<bool> = OnceLock::new();

/// True when this process runs from an installed MSIX package. Probed once.
pub fn is_packaged() -> bool {
    *PACKAGED.get_or_init(probe)
}

#[cfg(windows)]
fn probe() -> bool {
    use windows::core::PWSTR;
    use windows::Win32::Storage::Packaging::Appx::GetCurrentPackageFullName;
    const ERROR_INSUFFICIENT_BUFFER: u32 = 122;
    let mut len: u32 = 0;
    let rc = unsafe { GetCurrentPackageFullName(&mut len, PWSTR::null()) };
    // Sizing call: a packaged process answers ERROR_INSUFFICIENT_BUFFER (122) with the needed
    // length; an unpackaged one answers APPMODEL_ERROR_NO_PACKAGE (15700).
    rc.0 == ERROR_INSUFFICIENT_BUFFER && len > 0
}

#[cfg(not(windows))]
fn probe() -> bool { false }

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StartupState { Disabled, DisabledByUser, Enabled, DisabledByPolicy, EnabledByPolicy, Unavailable }

/// The app may only flip Enabled <-> Disabled. Everything else is Windows' or the user's call.
pub fn app_may_change(s: StartupState) -> bool { matches!(s, StartupState::Enabled | StartupState::Disabled) }

/// One sentence for the inert settings row. Empty when the row is live.
pub fn locked_note(s: StartupState) -> &'static str {
    match s {
        StartupState::Enabled | StartupState::Disabled => "",
        StartupState::DisabledByUser => "You turned TidyUp off in Task Manager's Startup apps, and Windows does not let an app switch itself back on. Turn it on there and this comes back.",
        StartupState::DisabledByPolicy => "Your organisation's policy decides this one, so the switch is off here.",
        StartupState::EnabledByPolicy => "Your organisation's policy starts TidyUp with Windows, so it cannot be turned off here.",
        StartupState::Unavailable => "Windows manages this for the Store version. Settings > Apps > Startup has the switch.",
    }
}

fn classify(raw: i32) -> StartupState {
    match raw { 0 => StartupState::Disabled, 1 => StartupState::DisabledByUser, 2 => StartupState::Enabled, 3 => StartupState::DisabledByPolicy, 4 => StartupState::EnabledByPolicy, _ => StartupState::Unavailable }
}

#[cfg(windows)]
fn get_task() -> windows::core::Result<windows::ApplicationModel::StartupTask> {
    use windows::core::HSTRING;
    windows::ApplicationModel::StartupTask::GetAsync(&HSTRING::from(STARTUP_TASK_ID))?.get()
}

#[cfg(windows)]
pub fn startup_task_state() -> StartupState {
    if !is_packaged() { return StartupState::Unavailable; }
    match get_task().and_then(|t| t.State()) { Ok(s) => classify(s.0), Err(e) => { log::warn!("startup task state: {e}"); StartupState::Unavailable } }
}

#[cfg(windows)]
pub fn set_startup_task(enabled: bool) -> StartupState {
    if !is_packaged() { return StartupState::Unavailable; }
    let Ok(task) = get_task() else { return StartupState::Unavailable };
    if enabled {
        match task.RequestEnableAsync().and_then(|op| op.get()) { Ok(s) => classify(s.0), Err(e) => { log::warn!("RequestEnableAsync: {e}"); StartupState::Unavailable } }
    } else {
        match task.Disable() { Ok(()) => StartupState::Disabled, Err(e) => { log::warn!("Disable: {e}"); StartupState::Unavailable } }
    }
}

#[cfg(not(windows))]
pub fn startup_task_state() -> StartupState { StartupState::Unavailable }
#[cfg(not(windows))]
pub fn set_startup_task(_enabled: bool) -> StartupState { StartupState::Unavailable }
