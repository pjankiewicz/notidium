//! Windows Task Scheduler (per-user logon task) backend.

use std::path::PathBuf;
use std::process::Command;

use crate::error::{Error, Result};

use super::{render, skip_daemon_start, ServiceManager, ServiceSpec, ServiceState};

const TASK_NAME: &str = "Notidium";

pub struct ScheduledTask;

fn local_appdata() -> Result<PathBuf> {
    let dir = std::env::var("LOCALAPPDATA")
        .map_err(|_| Error::Service("LOCALAPPDATA environment variable is not set".into()))?;
    Ok(PathBuf::from(dir))
}

fn xml_staging_path() -> Result<PathBuf> {
    Ok(local_appdata()?.join("Notidium").join("service").join("task.xml"))
}

fn current_user_id() -> Result<String> {
    // Prefer the domain\user form; fall back to just username. Both are
    // accepted by Task Scheduler's <UserId> element.
    let domain = std::env::var("USERDOMAIN").ok();
    let user = std::env::var("USERNAME")
        .map_err(|_| Error::Service("USERNAME environment variable is not set".into()))?;
    match domain {
        Some(d) if !d.is_empty() => Ok(format!("{d}\\{user}")),
        _ => Ok(user),
    }
}

fn task_exists() -> Result<bool> {
    let out = Command::new("schtasks.exe")
        .args(["/Query", "/TN", TASK_NAME])
        .output()?;
    Ok(out.status.success())
}

impl ServiceManager for ScheduledTask {
    fn install(&self, spec: &ServiceSpec, force: bool) -> Result<()> {
        if !force && task_exists()? {
            return Err(Error::Service(format!(
                "scheduled task `{TASK_NAME}` already exists. Run `notidium uninstall-service` first or pass --force."
            )));
        }

        if let Some(parent) = spec.log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let user = current_user_id()?;
        let xml = render::windows_task_xml(spec, &user);

        let xml_path = xml_staging_path()?;
        if let Some(parent) = xml_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Task Scheduler expects UTF-16 LE with BOM for XML import.
        let utf16: Vec<u16> = std::iter::once(0xFEFFu16)
            .chain(xml.encode_utf16())
            .collect();
        let bytes: Vec<u8> = utf16.iter().flat_map(|u| u.to_le_bytes()).collect();
        std::fs::write(&xml_path, bytes)?;

        if skip_daemon_start() {
            return Ok(());
        }

        let mut cmd = Command::new("schtasks.exe");
        cmd.args(["/Create", "/TN", TASK_NAME, "/XML"])
            .arg(&xml_path);
        if force {
            cmd.arg("/F");
        }
        let out = cmd.output()?;
        if !out.status.success() {
            return Err(Error::Service(format!(
                "schtasks /Create failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }

        // Kick off the first run immediately so the user doesn't have to log
        // out and back in to see the server come up.
        let _ = Command::new("schtasks.exe")
            .args(["/Run", "/TN", TASK_NAME])
            .status();

        Ok(())
    }

    fn uninstall(&self) -> Result<()> {
        if !task_exists()? {
            return Ok(());
        }
        let out = Command::new("schtasks.exe")
            .args(["/Delete", "/TN", TASK_NAME, "/F"])
            .output()?;
        if !out.status.success() {
            return Err(Error::Service(format!(
                "schtasks /Delete failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        Ok(())
    }

    fn status(&self) -> Result<ServiceState> {
        if !task_exists()? {
            return Ok(ServiceState::NotInstalled);
        }

        let out = Command::new("schtasks.exe")
            .args(["/Query", "/TN", TASK_NAME, "/FO", "CSV", "/NH"])
            .output()?;

        if !out.status.success() {
            return Ok(ServiceState::Unknown);
        }

        let stdout = String::from_utf8_lossy(&out.stdout);
        let line = stdout.lines().next().unwrap_or("");
        // CSV columns: "TaskName","Next Run Time","Status"
        let last_field = line.rsplit(',').next().unwrap_or("").trim_matches('"');

        match last_field {
            "Running" => Ok(ServiceState::Running),
            "Ready" | "Queued" => Ok(ServiceState::Stopped),
            "Disabled" => Ok(ServiceState::Stopped),
            other if other.is_empty() => Ok(ServiceState::Unknown),
            other => Ok(ServiceState::Failed(other.to_string())),
        }
    }
}
