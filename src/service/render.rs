//! Pure renderers for per-platform service unit files.
//!
//! These functions take a [`ServiceSpec`] and produce the exact text that
//! should be written to disk. They do not touch the filesystem or invoke any
//! platform tooling, which keeps them unit-testable on any OS.

use super::ServiceSpec;

/// macOS launchd property list (XML plist).
///
/// `RunAtLoad=true` + `KeepAlive=true` gives us the same
/// "start at login, restart on crash" behavior as the other backends.
/// `StandardOutPath`/`StandardErrorPath` redirect the server's stdio to the
/// vault log path, matching the existing `Config::logs_path` layout.
pub fn launchd_plist(spec: &ServiceSpec) -> String {
    let log = xml_escape(&spec.log_path.to_string_lossy());
    let binary = xml_escape(&spec.binary_path.to_string_lossy());
    let vault = xml_escape(&spec.vault_path.to_string_lossy());
    let port = spec.port;

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>dev.notidium.server</string>
    <key>ProgramArguments</key>
    <array>
        <string>{binary}</string>
        <string>serve</string>
        <string>{vault}</string>
        <string>-p</string>
        <string>{port}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{log}</string>
    <key>StandardErrorPath</key>
    <string>{log}</string>
    <key>ProcessType</key>
    <string>Background</string>
</dict>
</plist>
"#
    )
}

/// User-level systemd unit file.
///
/// `StandardOutput=append:` and `StandardError=append:` (systemd >= 240) write
/// the server's stdio to the vault log path. `Restart=on-failure` gives us the
/// same crash-recovery behavior as the other backends.
pub fn systemd_unit(spec: &ServiceSpec) -> String {
    let binary = spec.binary_path.to_string_lossy();
    let vault = spec.vault_path.to_string_lossy();
    let log = spec.log_path.to_string_lossy();
    let port = spec.port;

    format!(
        r#"[Unit]
Description=Notidium server (local-first notes)
After=network.target

[Service]
Type=simple
ExecStart={binary} serve {vault} -p {port}
Restart=on-failure
RestartSec=5
StandardOutput=append:{log}
StandardError=append:{log}

[Install]
WantedBy=default.target
"#
    )
}

/// Windows Task Scheduler XML for a per-user logon task.
///
/// The task runs under the current user with least-privilege. `cmd.exe` is
/// used as a wrapper purely to redirect stdout/stderr to the log file, since
/// Task Scheduler's `<Exec>` action does not natively capture stdio.
/// `RestartOnFailure` gives us crash recovery similar to launchd `KeepAlive`.
pub fn windows_task_xml(spec: &ServiceSpec, user_id: &str) -> String {
    let binary = xml_escape(&spec.binary_path.to_string_lossy());
    let vault = xml_escape(&spec.vault_path.to_string_lossy());
    let log = xml_escape(&spec.log_path.to_string_lossy());
    let user = xml_escape(user_id);
    let port = spec.port;

    let wrapped_args = format!(
        "/c \"\"{binary}\" serve \"{vault}\" -p {port} &gt; \"{log}\" 2&gt;&amp;1\""
    );

    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.4" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>Notidium server (local-first notes)</Description>
    <URI>\Notidium</URI>
  </RegistrationInfo>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <UserId>{user}</UserId>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>{user}</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
    <Enabled>true</Enabled>
    <Hidden>false</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle>
    <WakeToRun>false</WakeToRun>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>7</Priority>
    <RestartOnFailure>
      <Interval>PT1M</Interval>
      <Count>3</Count>
    </RestartOnFailure>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>cmd.exe</Command>
      <Arguments>{wrapped_args}</Arguments>
    </Exec>
  </Actions>
</Task>
"#
    )
}

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn spec() -> ServiceSpec {
        ServiceSpec {
            binary_path: PathBuf::from("/home/alice/.notidium/bin/notidium"),
            vault_path: PathBuf::from("/home/alice/Notidium"),
            port: 3939,
            log_path: PathBuf::from("/home/alice/Notidium/.notidium/logs/server.log"),
        }
    }

    #[test]
    fn launchd_plist_contains_expected_keys() {
        let out = launchd_plist(&spec());
        assert!(out.contains("<key>Label</key>"));
        assert!(out.contains("<string>dev.notidium.server</string>"));
        assert!(out.contains("<key>RunAtLoad</key>"));
        assert!(out.contains("<key>KeepAlive</key>"));
        assert!(out.contains("/home/alice/.notidium/bin/notidium"));
        assert!(out.contains("<string>serve</string>"));
        assert!(out.contains("<string>3939</string>"));
        assert!(out.contains("/home/alice/Notidium/.notidium/logs/server.log"));
    }

    #[test]
    fn launchd_plist_escapes_xml_special_chars_in_paths() {
        let mut s = spec();
        s.vault_path = PathBuf::from("/tmp/a&b<c>\"d");
        let out = launchd_plist(&s);
        assert!(out.contains("/tmp/a&amp;b&lt;c&gt;&quot;d"));
        assert!(!out.contains("/tmp/a&b<c>"));
    }

    #[test]
    fn systemd_unit_has_service_and_install_sections() {
        let out = systemd_unit(&spec());
        assert!(out.contains("[Unit]"));
        assert!(out.contains("[Service]"));
        assert!(out.contains("[Install]"));
        assert!(out.contains("ExecStart=/home/alice/.notidium/bin/notidium serve /home/alice/Notidium -p 3939"));
        assert!(out.contains("Restart=on-failure"));
        assert!(out.contains("StandardOutput=append:/home/alice/Notidium/.notidium/logs/server.log"));
        assert!(out.contains("WantedBy=default.target"));
    }

    #[test]
    fn windows_task_xml_embeds_logon_trigger_and_user() {
        let out = windows_task_xml(&spec(), "DESKTOP-ABC\\alice");
        assert!(out.contains("<LogonTrigger>"));
        assert!(out.contains("<UserId>DESKTOP-ABC\\alice</UserId>"));
        assert!(out.contains("<Command>cmd.exe</Command>"));
        assert!(out.contains("serve"));
        assert!(out.contains("-p 3939"));
        assert!(out.contains("<RestartOnFailure>"));
        assert!(out.contains("<Count>3</Count>"));
    }

    #[test]
    fn windows_task_xml_escapes_redirection_operators_in_attribute() {
        let out = windows_task_xml(&spec(), "alice");
        // The redirection operators must appear as XML entities, not raw.
        assert!(out.contains("&gt;"));
        assert!(out.contains("2&gt;&amp;1"));
        // No unescaped `>` inside the <Arguments> element beyond the closing tag.
        let args_start = out.find("<Arguments>").unwrap();
        let args_end = out[args_start..].find("</Arguments>").unwrap() + args_start;
        let args_inner = &out[args_start + "<Arguments>".len()..args_end];
        assert!(!args_inner.contains('>'));
    }
}
