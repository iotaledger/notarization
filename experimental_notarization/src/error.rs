

/// Errors that can occur when managing Notarizations
#[derive(Debug, thiserror::Error, strum::IntoStaticStr)]
#[non_exhaustive]
pub enum Error {
  /// Caused by invalid keys.
  #[error("invalid key: {0}")]
  InvalidKey(String),
}