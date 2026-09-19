use std::str::FromStr;

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

use crate::error::LedgerError;

/// Opens a connection pool to the SQLite database at `url`, creating the file
/// if it does not exist, and applies any pending migrations.
///
/// Accepts URLs such as `sqlite://ledger.db` or `sqlite::memory:`.
pub async fn connect(url: &str) -> Result<SqlitePool, LedgerError> {
    let options = SqliteConnectOptions::from_str(url)?
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new().connect_with(options).await?;
    migrate(&pool).await?;
    Ok(pool)
}

/// Applies any pending schema migrations to the database.
pub async fn migrate(pool: &SqlitePool) -> Result<(), LedgerError> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}
