use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeHostConfig {
    pub libnode_path: PathBuf,
    pub bootstrap_path: PathBuf,
    pub request_timeout: Duration,
}

impl NodeHostConfig {
    pub fn new(libnode_path: impl Into<PathBuf>, bootstrap_path: impl Into<PathBuf>) -> Self {
        Self {
            libnode_path: libnode_path.into(),
            bootstrap_path: bootstrap_path.into(),
            request_timeout: Duration::from_secs(30),
        }
    }

    pub fn from_env(bootstrap_path: impl Into<PathBuf>) -> crate::Result<Self> {
        let libnode_path = std::env::var_os("PI_GPUI_LIBNODE")
            .or_else(|| std::env::var_os("EDON_LIBNODE_PATH"))
            .map(PathBuf::from)
            .ok_or(crate::NodeHostError::MissingLibnode)?;
        Ok(Self::new(libnode_path, bootstrap_path))
    }
}
