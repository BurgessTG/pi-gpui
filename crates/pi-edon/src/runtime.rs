use std::path::{Path, PathBuf};

use crate::{EdonBoundaryError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedNodeConfig {
    pub libnode_path: PathBuf,
    pub disable_warnings: Vec<String>,
}

impl EmbeddedNodeConfig {
    pub fn from_env() -> Result<Self> {
        let path = std::env::var_os("PI_GPUI_LIBNODE")
            .or_else(|| std::env::var_os("EDON_LIBNODE_PATH"))
            .map(PathBuf::from)
            .ok_or(EdonBoundaryError::MissingLibnode)?;
        Ok(Self::new(path))
    }

    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            libnode_path: path.into(),
            disable_warnings: vec!["ExperimentalWarning".to_owned()],
        }
    }
}

pub struct EmbeddedNode {
    inner: edon::Nodejs,
}

impl EmbeddedNode {
    pub fn load(config: EmbeddedNodeConfig) -> Result<Self> {
        let inner = edon::Nodejs::load(edon::NodejsOptions {
            libnode_path: config.libnode_path,
            disable_warnings: config.disable_warnings,
            ..edon::NodejsOptions::default()
        })?;
        Ok(Self { inner })
    }

    pub fn load_from_env() -> Result<Self> {
        Self::load(EmbeddedNodeConfig::from_env()?)
    }

    pub fn register_module<F>(&self, module_name: &str, register: F) -> Result<()>
    where
        F: 'static
            + Sync
            + Send
            + Fn(edon::Env, edon::napi::JsObject) -> edon::Result<edon::napi::JsObject>,
    {
        self.inner.napi_module_register(module_name, register)?;
        Ok(())
    }

    pub fn eval(&self, code: &str) -> Result<()> {
        self.inner.eval_blocking(code)?;
        Ok(())
    }

    pub fn eval_typescript(&self, code: &str) -> Result<()> {
        self.inner.eval_typescript_blocking(code)?;
        Ok(())
    }

    pub fn import(&self, specifier: &str) -> Result<()> {
        self.inner.import(specifier)?;
        Ok(())
    }

    pub fn require(&self, specifier: &str) -> Result<()> {
        self.inner.require(specifier)?;
        Ok(())
    }

    pub fn libnode_exists(path: &Path) -> bool {
        path.is_file()
    }
}
