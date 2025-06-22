use crate::error::{Error, Result};
use crate::i18n::{t, tr};
use crate::ui::{spin_frame, ProgressBar};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Try installing via Windows `winget` with an animated progress bar.
/// Returns Ok(true) if install appeared successful, Ok(false) if winget missing.
pub fn try_winget_install(package_id: &str) -> Result<bool> {
    let has = Command::new("winget")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !has {
        return Ok(false);
    }

    let bar = ProgressBar::new(tr("provision.downloading", &[("pkg", package_id)]));
    let mut child = Command::new("winget")
        .args([
            "install",
            "-e",
            "--id",
            package_id,
            "--accept-package-agreements",
            "--accept-source-agreements",
            "--disable-interactivity",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Error::msg(tr("provision.download_err", &[("error", &e.to_string())])))?;

    let stdout = child.stdout.take();
    let drain = thread::spawn(move || {
        if let Some(out) = stdout {
            for line in BufReader::new(out).lines().flatten() {
                let _ = line;
            }
        }
    });

    let start = Instant::now();
    let mut tick = 0usize;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let _ = drain.join();
                if status.success() {
                    let _ = bar.finish(&t("provision.download_done"));
                    return Ok(true);
                }
                let _ = bar.fail(&t("provision.download_fail"));
                return Err(Error::msg(t("provision.download_fail")));
            }
            Ok(None) => {
                let elapsed = start.elapsed().as_secs_f64();
                let frac = (1.0 - (-elapsed / 45.0).exp()).min(0.97);
                let _ = bar.render(frac, spin_frame(tick));
                tick += 1;
                thread::sleep(Duration::from_millis(150));
            }
            Err(e) => {
                let _ = drain.join();
                let _ = bar.fail(&e.to_string());
                return Err(Error::msg(e.to_string()));
            }
        }
    }
}
