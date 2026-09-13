use serde::Deserialize;

pub const DEFAULT_HOST: &str = "127.0.0.1";
pub const DEFAULT_PORT: u16 = 8888;
pub const DEFAULT_TIMEOUT_MS: u64 = 5000;

#[derive(Debug, Deserialize)]
#[serde(tag = "request", rename_all = "lowercase")]
pub enum AdapterConfig {
    Launch {
        host: Option<String>,
        port: Option<u16>,
        project_root_dir: Option<String>,
        program: String,
        lua: Option<String>,

        #[serde(default)]
        args: Vec<String>,
    },
    Attach {
        host: Option<String>,
        port: Option<u16>,
        project_root_dir: Option<String>,
    },
}

impl AdapterConfig {
    pub fn host(&self) -> Option<String> {
        match self {
            AdapterConfig::Launch { host, .. } | AdapterConfig::Attach { host, .. } => host.clone(),
        }
    }

    pub fn port(&self) -> Option<u16> {
        match self {
            AdapterConfig::Launch { port, .. } | AdapterConfig::Attach { port, .. } => *port,
        }
    }

    pub fn project_root_dir(&self) -> Option<String> {
        match self {
            AdapterConfig::Launch {
                project_root_dir, ..
            }
            | AdapterConfig::Attach {
                project_root_dir, ..
            } => project_root_dir.clone(),
        }
    }
}
