use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum TexError {
    #[error("container not found: {0}")]
    ContainerNotFound(PathBuf),
    #[error("usmap mappings not found: {0}")]
    UsmapNotFound(PathBuf),
    #[error("asset not found in container: {0}")]
    AssetNotFound(String),
    #[error("unsupported pixel format: {0}")]
    UnsupportedFormat(String),
    #[error("virtual textures are not supported in v1: {0}")]
    VirtualTexture(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("container read/parse error: {0}")]
    Retoc(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, TexError>;
