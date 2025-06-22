//! Local provisioning: Docker pull/run and optional winget download.

mod docker;
mod download;

pub use docker::{docker_available, pull_and_run, DockerRunResult};
pub use download::try_winget_install;
