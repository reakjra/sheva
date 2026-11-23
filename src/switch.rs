use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail, ensure};
use nix::unistd::geteuid;

const DISABLED_FLAG: &str = "/etc/sheva/disabled";

pub fn set(enabled: bool) -> Result<()> {
    ensure!(geteuid().is_root(), "this needs root, run it with sudo");
    let flag = Path::new(DISABLED_FLAG);
    if enabled {
        match fs::remove_file(flag) {
            Err(err) if err.kind() != ErrorKind::NotFound => {
                return Err(err).with_context(|| format!("removing {DISABLED_FLAG}"));
            }
            _ => {}
        }
    } else {
        if let Some(dir) = flag.parent() {
            fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        fs::write(flag, "").with_context(|| format!("creating {DISABLED_FLAG}"))?;
        run("systemctl", &["stop", "sheva@*.service"])?;
    }
    run(
        "udevadm",
        &[
            "trigger",
            "--action=add",
            "--subsystem-match=hidraw",
            "--subsystem-match=input",
        ],
    )
}

fn run(program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("running {program}"))?;
    if !status.success() {
        bail!("{program} {} failed ({status})", args.join(" "));
    }
    Ok(())
}
