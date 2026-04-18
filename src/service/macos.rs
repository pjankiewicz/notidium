//! macOS launchd user agent backend.

use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::error::{Error, Result};

use super::{render, skip_daemon_start, ServiceManager, ServiceSpec, ServiceState};

const LABEL: &str = "dev.notidium.server";

pub struct LaunchdAgent;

fn plist_path() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| Error::Service("could not locate user home directory".into()))?;
    Ok(home
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{LABEL}.plist")))
}

fn service_target() -> Result<String> {
    // `gui/<uid>` is the per-user launchd domain for modern macOS.
    let uid = unsafe { libc_getuid() };
    Ok(format!("gui/{uid}/{LABEL}"))
}

fn gui_domain() -> Result<String> {
    let uid = unsafe { libc_getuid() };
    Ok(format!("gui/{uid}"))
}

// Avoid pulling in the `libc` crate just for getuid; `id -u` works on every mac.
// Using a tiny unsafe wrapper keeps this crate's dep set unchanged.
#[allow(non_snake_case)]
unsafe fn libc_getuid() -> u32 {
    extern "C" {
        fn getuid() -> u32;
    }
    getuid()
}

impl ServiceManager for LaunchdAgent {
    fn install(&self, spec: &ServiceSpec, force: bool) -> Result<()> {
        let path = plist_path()?;
        if path.exists() && !force {
            return Err(Error::Service(format!(
                "launchd agent already installed at {}. Run `notidium uninstall-service` first or pass --force.",
                path.display()
            )));
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if let Some(parent) = spec.log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let contents = render::launchd_plist(spec);
        std::fs::write(&path, contents)?;

        if skip_daemon_start() {
            return Ok(());
        }

        // Unload any prior instance before loading the new one. Failures here
        // are expected (no prior load) — swallow stdout/stderr so they don't
        // pollute normal output.
        let _ = Command::new("launchctl")
            .args(["bootout", &service_target()?])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        let status = Command::new("launchctl")
            .args(["bootstrap", &gui_domain()?])
            .arg(&path)
            .status()?;

        if !status.success() {
            // Fallback for older macOS versions without `bootstrap`.
            let fallback = Command::new("launchctl")
                .arg("load")
                .arg(&path)
                .status()?;
            if !fallback.success() {
                return Err(Error::Service(
                    "launchctl failed to load the agent; check the plist path and permissions".into(),
                ));
            }
        }

        Ok(())
    }

    fn uninstall(&self) -> Result<()> {
        let path = plist_path()?;
        if !path.exists() {
            return Ok(());
        }

        if !skip_daemon_start() {
            let _ = Command::new("launchctl")
                .args(["bootout", &service_target()?])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            let _ = Command::new("launchctl")
                .arg("unload")
                .arg(&path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }

        std::fs::remove_file(&path)?;
        Ok(())
    }

    fn status(&self) -> Result<ServiceState> {
        let path = plist_path()?;
        if !path.exists() {
            return Ok(ServiceState::NotInstalled);
        }

        let output = Command::new("launchctl")
            .args(["print", &service_target()?])
            .output()?;

        if !output.status.success() {
            return Ok(ServiceState::Stopped);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("state = running") {
            Ok(ServiceState::Running)
        } else if let Some(idx) = stdout.find("last exit code = ") {
            let rest = &stdout[idx + "last exit code = ".len()..];
            let code = rest.split_whitespace().next().unwrap_or("?");
            if code == "0" {
                Ok(ServiceState::Stopped)
            } else {
                Ok(ServiceState::Failed(format!("last exit code {code}")))
            }
        } else {
            Ok(ServiceState::Unknown)
        }
    }
}
