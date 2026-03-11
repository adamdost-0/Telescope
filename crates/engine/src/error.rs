//! Engine error types.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Kubeconfig error: {0}")]
    Kubeconfig(#[from] kube::config::KubeconfigError),

    #[error("Kube client error: {0}")]
    Kube(#[from] kube::Error),

    #[error("Kube config error: {0}")]
    Config(#[from] kube::config::InferConfigError),

    #[error("Store error: {0}")]
    Store(String),

    #[error("No active context set")]
    NoActiveContext,

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, EngineError>;
