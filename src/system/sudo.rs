use std::io::Write;
use std::process::{Command, Stdio};

pub fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

pub fn validate_sudo_password(password: &str) -> bool {
    if is_root() {
        return true;
    }
    if password.is_empty() {
        return false;
    }

    let mut child = match Command::new("sudo")
        .args(["-S", "-p", "", "-v"])
        .env("DEBIAN_FRONTEND", "noninteractive")
        .env("SYSTEMD_PAGER", "")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(password.as_bytes());
        let _ = stdin.write_all(b"\n");
        let _ = stdin.flush();
    }

    match child.wait() {
        Ok(status) => status.success(),
        Err(_) => false,
    }
}

pub fn run_elevated_command(cmd: &str, args: &[&str], sudo_pass: Option<&str>) -> Result<String, String> {
    if is_root() {
        let output = Command::new(cmd)
            .args(args)
            .env("DEBIAN_FRONTEND", "noninteractive")
            .env("SYSTEMD_PAGER", "")
            .env("PAGER", "cat")
            .output()
            .map_err(|e| format!("Execution failed: {}", e))?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            Err(if err.trim().is_empty() {
                format!("Command exited with code: {:?}", output.status.code())
            } else {
                err.trim().to_string()
            })
        }
    } else if let Some(pass) = sudo_pass {
        let mut sudo_args = vec!["-S", "-p", "", cmd];
        sudo_args.extend_from_slice(args);

        let mut child = Command::new("sudo")
            .args(&sudo_args)
            .env("DEBIAN_FRONTEND", "noninteractive")
            .env("SYSTEMD_PAGER", "")
            .env("PAGER", "cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn sudo: {}", e))?;

        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(pass.as_bytes());
            let _ = stdin.write_all(b"\n");
            let _ = stdin.flush();
        }

        let output = child.wait_with_output().map_err(|e| format!("Wait failed: {}", e))?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            Err(if err.trim().is_empty() {
                format!("Sudo command exited with code: {:?}", output.status.code())
            } else {
                err.trim().to_string()
            })
        }
    } else {
        Err("Superuser (sudo) password required".to_string())
    }
}
