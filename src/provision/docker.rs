use crate::db::DockerSpec;
use crate::error::{Error, Result};
use crate::i18n::{t, tr};
use crate::ui::{spin_frame, ProgressBar};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub struct DockerRunResult {
    pub container_name: String,
    pub url: String,
    pub host: String,
    pub port: u16,
}

pub fn docker_available() -> bool {
    Command::new("docker")
        .arg("version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn pull_and_run(spec: &DockerSpec, db_label: &str) -> Result<DockerRunResult> {
    pull_image(spec.image)?;
    run_container(spec, db_label)
}

fn pull_image(image: &str) -> Result<()> {
    let bar = ProgressBar::new(tr("provision.pulling", &[("image", image)]));
    let mut child = Command::new("docker")
        .args(["pull", image])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Error::msg(tr("provision.docker_err", &[("error", &e.to_string())])))?;

    let stderr = child.stderr.take();
    let start = Instant::now();
    let mut tick = 0usize;

    // docker pull progress is on stderr; animate while process runs
    let handle = thread::spawn(move || {
        if let Some(err) = stderr {
            let reader = BufReader::new(err);
            for line in reader.lines().flatten() {
                let _ = line; // keep draining
            }
        }
    });

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let _ = handle.join();
                if status.success() {
                    let _ = bar.finish(&t("provision.pull_done"));
                    return Ok(());
                }
                let _ = bar.fail(&t("provision.pull_fail"));
                return Err(Error::msg(t("provision.pull_fail")));
            }
            Ok(None) => {
                let elapsed = start.elapsed().as_secs_f64();
                // Soft asymptotic progress so the bar moves without lying at 100%
                let frac = 1.0 - (-elapsed / 25.0).exp();
                let _ = bar.render(
                    frac.min(0.95),
                    &format!("{} {}", spin_frame(tick), image),
                );
                tick += 1;
                thread::sleep(Duration::from_millis(120));
            }
            Err(e) => {
                let _ = handle.join();
                let _ = bar.fail(&e.to_string());
                return Err(Error::msg(e.to_string()));
            }
        }
    }
}

fn run_container(spec: &DockerSpec, db_label: &str) -> Result<DockerRunResult> {
    let safe: String = db_label
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let name = format!("{}-{}", spec.name_prefix, safe.to_ascii_lowercase());
    // Remove leftover container with same name (ignore errors)
    let _ = Command::new("docker")
        .args(["rm", "-f", &name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    let mut args = vec!["run".into(), "-d".into(), "--name".into(), name.clone()];
    for (k, v) in spec.env {
        args.push("-e".into());
        args.push(format!("{k}={v}"));
    }
    let mut host_port = 0u16;
    for (host, container) in spec.ports {
        host_port = *host;
        args.push("-p".into());
        args.push(format!("{host}:{container}"));
    }
    args.push(spec.image.to_string());

    let status = Command::new("docker")
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .status()
        .map_err(|e| Error::msg(tr("provision.docker_err", &[("error", &e.to_string())])))?;
    if !status.success() {
        return Err(Error::msg(t("provision.docker_run_fail")));
    }

    // Wait until port accepts connections
    let bar = ProgressBar::new(t("provision.waiting"));
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut tick = 0usize;
    while Instant::now() < deadline {
        if !matches!(
            crate::db::probe::tcp_localhost(host_port),
            crate::db::LocalStatus::Missing
        ) {
            let _ = bar.finish(&t("provision.server_ready"));
            let host = "127.0.0.1";
            let url = spec
                .url_template
                .replace("{host}", host)
                .replace("{port}", &host_port.to_string());
            return Ok(DockerRunResult {
                container_name: name,
                url,
                host: host.into(),
                port: host_port,
            });
        }
        let _ = bar.render(
            0.3 + (tick as f64 % 40.0) / 100.0,
            spin_frame(tick),
        );
        tick += 1;
        thread::sleep(Duration::from_millis(400));
    }
    let _ = bar.fail(&t("provision.wait_timeout"));
    Err(Error::msg(t("provision.wait_timeout")))
}
