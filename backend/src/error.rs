use thiserror::Error;

use crate::validation::ValidationError;

/// Errors that can occur while persisting or loading ledger data.
#[derive(Debug, Error)]
pub enum LedgerError {
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("invalid data stored in database: {0}")]
    CorruptData(String),
    #[error("{0} not found")]
    NotFound(String),
}
