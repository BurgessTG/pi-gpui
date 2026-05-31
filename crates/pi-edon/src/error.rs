pub type Result<T> = std::result::Result<T, EdonBoundaryError>;

#[derive(Debug, thiserror::Error)]
pub enum EdonBoundaryError {
    #[error("libnode path is not configured; set PI_GPUI_LIBNODE or EDON_LIBNODE_PATH")]
    MissingLibnode,
    #[error("edon/libnode error: {0}")]
    Edon(String),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<edon::Error> for EdonBoundaryError {
    fn from(value: edon::Error) -> Self {
        Self::Edon(value.to_string())
    }
}
