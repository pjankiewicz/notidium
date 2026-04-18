//! Auto-start service installation for Notidium.
//!
//! Each platform has its own native mechanism for running a program at login:
//! - macOS: user-level launchd agent (`~/Library/LaunchAgents/*.plist`)
//! - Linux: user systemd unit (`~/.config/systemd/user/*.service`)
//! - Windows: per-user Task Scheduler logon task
//!
//! The [`ServiceManager`] trait abstracts over these; [`current`] returns the
//! backend for the running platform. Unit/plist/XML generation is kept in
//! [`render`] as pure functions so it can be unit-tested on any host.

use std::path::PathBuf;

use crate::error::Result;

pub mod render;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

/// Environment variable honored by service backends to skip the final
/// activation step (launchctl / systemctl / schtasks). Used by CI smoke
/// tests so the service file can be generated and written without actually
/// starting a daemon.
pub const SKIP_DAEMON_ENV: &str = "NOTIDIUM_SKIP_DAEMON_START";

/// Inputs needed to render a service unit. All paths must be absolute.
#[derive(Debug, Clone)]
pub struct ServiceSpec {
    pub binary_path: PathBuf,
    pub vault_path: PathBuf,
    pub port: u16,
    pub log_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceState {
    NotInstalled,
    Stopped,
    Running,
    Failed(String),
    Unknown,
}

pub trait ServiceManager {
    fn install(&self, spec: &ServiceSpec, force: bool) -> Result<()>;
    fn uninstall(&self) -> Result<()>;
    fn status(&self) -> Result<ServiceState>;
    fn log_path(&self, spec: &ServiceSpec) -> PathBuf {
        spec.log_path.clone()
    }
}

/// Returns the service manager for the current platform.
pub fn current() -> Box<dyn ServiceManager> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::LaunchdAgent)
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::SystemdUserUnit)
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::ScheduledTask)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        compile_error!("notidium service install is only supported on macOS, Linux, and Windows")
    }
}

/// Returns true if the environment indicates we should skip actually
/// starting the daemon (used in CI smoke tests).
pub fn skip_daemon_start() -> bool {
    std::env::var(SKIP_DAEMON_ENV).is_ok_and(|v| !v.is_empty() && v != "0")
}
