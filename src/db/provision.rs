//! How a driver can be created / provisioned on the local machine.

/// How new databases of this type are created.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateKind {
    /// Local file (SQLite).
    File,
    /// Needs a running server process.
    Server,
}

#[derive(Debug, Clone)]
pub struct DockerSpec {
    pub image: &'static str,
    /// `-e KEY=VALUE`
    pub env: &'static [(&'static str, &'static str)],
    /// host_port → container_port
    pub ports: &'static [(u16, u16)],
    /// Optional container name prefix
    pub name_prefix: &'static str,
    /// Connection URL template after start; `{host}` `{port}` `{user}` `{password}` `{db}`
    pub url_template: &'static str,
}

#[derive(Debug, Clone)]
pub struct ProvisionInfo {
    pub create_kind: CreateKind,
    pub default_port: Option<u16>,
    pub docker: Option<DockerSpec>,
    /// Official download / docs page when offline install is needed.
    pub download_url: Option<&'static str>,
    /// `winget` package id when available (Windows).
    pub winget_id: Option<&'static str>,
    /// Short human label for create help (i18n key preferred in UI).
    pub help_key: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalStatus {
    /// Something already listening / usable on localhost.
    Ready { host: String, port: u16 },
    /// No local server detected.
    Missing,
}

impl Default for ProvisionInfo {
    fn default() -> Self {
        Self {
            create_kind: CreateKind::File,
            default_port: None,
            docker: None,
            download_url: None,
            winget_id: None,
            help_key: "provision.help.generic",
        }
    }
}
