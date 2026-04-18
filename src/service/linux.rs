//! Linux user-systemd backend.

use std::path::PathBuf;
use std::process::Command;

use crate::error::{Error, Result};

use super::{render, skip_daemon_start, ServiceManager, ServiceSpec, ServiceState};

const UNIT_NAME: &str = "notidium.service";

pub struct SystemdUserUnit;

fn unit_path() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| Error::Service("could not locate user home directory".into()))?;
    Ok(home.join(".config").join("systemd").join("user").join(UNIT_NAME))
}

fn systemctl_user(args: &[&str]) -> Result<std::process::Output> {
    let mut cmd = Command::new("systemctl");
    cmd.arg("--user");
    cmd.args(args);
    Ok(cmd.output()?)
}

impl ServiceManager for SystemdUserUnit {
    fn install(&self, spec: &ServiceSpec, force: bool) -> Result<()> {
        let path = unit_path()?;
        if path.exists() && !force {
            return Err(Error::Service(format!(
                "systemd unit already installed at {}. Run `notidium uninstall-service` first or pass --force.",
                path.display()
            )));
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if let Some(parent) = spec.log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let contents = render::systemd_unit(spec);
        std::fs::write(&path, contents)?;

        if skip_daemon_start() {
            return Ok(());
        }

        let reload = systemctl_user(&["daemon-reload"])?;
        if !reload.status.success() {
            return Err(Error::Service(format!(
                "systemctl --user daemon-reload failed: {}",
                String::from_utf8_lossy(&reload.stderr).trim()
            )));
        }

        let enable = systemctl_user(&["enable", "--now", UNIT_NAME])?;
        if !enable.status.success() {
            return Err(Error::Service(format!(
                "systemctl --user enable --now {UNIT_NAME} failed: {}",
                String::from_utf8_lossy(&enable.stderr).trim()
            )));
        }

        Ok(())
    }

    fn uninstall(&self) -> Result<()> {
        let path = unit_path()?;
        if !path.exists() {
            return Ok(());
        }

        if !skip_daemon_start() {
            let _ = systemctl_user(&["disable", "--now", UNIT_NAME])?;
        }

        std::fs::remove_file(&path)?;

        if !skip_daemon_start() {
            let _ = systemctl_user(&["daemon-reload"])?;
        }

        Ok(())
    }

    fn status(&self) -> Result<ServiceState> {
        let path = unit_path()?;
        if !path.exists() {
            return Ok(ServiceState::NotInstalled);
        }

        let out = systemctl_user(&["is-active", UNIT_NAME])?;
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();

        match text.as_str() {
            "active" => Ok(ServiceState::Running),
            "inactive" => Ok(ServiceState::Stopped),
            "failed" => Ok(ServiceState::Failed("systemd reports 'failed'".into())),
            "" => Ok(ServiceState::Unknown),
            other => Ok(ServiceState::Failed(other.to_string())),
        }
    }
}
