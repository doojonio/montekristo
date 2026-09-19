use std::collections::HashMap;
use std::str::FromStr;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::db;
use crate::error::LedgerError;
use crate::models::{Account, Posting, Transaction};

/// SQLite-backed repository for the ledger.
#[derive(Debug, Clone)]
pub struct Ledger {
    pool: SqlitePool,
}

impl Ledger {
    /// Connects to the database at `url` (creating it if necessary) and runs
    /// pending migrations. See [`db::connect`].
    pub async fn connect(url: &str) -> Result<Self, LedgerError> {
        Ok(Self::new(db::connect(url).await?))
    }

    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Persists a new account.
    pub async fn insert_account(&self, account: &Account) -> Result<(), LedgerError> {
        sqlx::query("INSERT INTO accounts (id, name, account_type, currency) VALUES (?, ?, ?, ?)")
            .bind(account.id.to_string())
            .bind(&account.name)
            .bind(account.account_type)
            .bind(&account.currency)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Validates a transaction and persists it together with its postings
    /// atomically, inside a single database transaction.
    pub async fn insert_transaction(&self, transaction: &Transaction) -> Result<(), LedgerError> {
        transaction.validate()?;

        let mut tx = self.pool.begin().await?;
        sqlx::query("INSERT INTO transactions (id, date, description) VALUES (?, ?, ?)")
            .bind(transaction.id.to_string())
            .bind(transaction.date)
            .bind(&transaction.description)
            .execute(&mut *tx)
            .await?;

        for posting in &transaction.postings {
            sqlx::query(
                "INSERT INTO postings (transaction_id, account_id, amount) VALUES (?, ?, ?)",
            )
            .bind(transaction.id.to_string())
            .bind(posting.account_id.to_string())
            .bind(posting.amount.to_string())
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Returns all accounts, ordered by name.
    pub async fn accounts(&self) -> Result<Vec<Account>, LedgerError> {
        sqlx::query_as::<_, AccountRow>(
            "SELECT id, name, account_type, currency FROM accounts ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(Account::try_from)
        .collect()
    }

    /// Returns all transactions with their postings, ordered by date then id.
    /// Postings retain their original insertion order.
    pub async fn transactions(&self) -> Result<Vec<Transaction>, LedgerError> {
        let tx_rows = sqlx::query_as::<_, TransactionRow>(
            "SELECT id, date, description FROM transactions ORDER BY date, id",
        )
        .fetch_all(&self.pool)
        .await?;

        let posting_rows = sqlx::query_as::<_, PostingRow>(
            "SELECT transaction_id, account_id, amount FROM postings ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut postings_by_tx: HashMap<Uuid, Vec<Posting>> = HashMap::new();
        for row in posting_rows {
            let transaction_id = parse_uuid(&row.transaction_id, "posting transaction_id")?;
            let amount = Decimal::from_str(&row.amount).map_err(|e| {
                LedgerError::CorruptData(format!("posting amount {:?}: {e}", row.amount))
            })?;
            postings_by_tx
                .entry(transaction_id)
                .or_default()
                .push(Posting {
                    account_id: parse_uuid(&row.account_id, "posting account_id")?,
                    amount,
                });
        }

        tx_rows
            .into_iter()
            .map(|row| {
                let id = parse_uuid(&row.id, "transaction id")?;
                Ok(Transaction {
                    id,
                    date: row.date,
                    description: row.description,
                    postings: postings_by_tx.remove(&id).unwrap_or_default(),
                })
            })
            .collect()
    }
}

fn parse_uuid(value: &str, field: &str) -> Result<Uuid, LedgerError> {
    Uuid::parse_str(value).map_err(|e| LedgerError::CorruptData(format!("{field} {value:?}: {e}")))
}

/// Ids are stored as hyphenated uuid text; sqlx decodes raw `Uuid` only from
/// 16-byte blobs, so rows carry `String` ids parsed via [`parse_uuid`].
#[derive(FromRow)]
struct AccountRow {
    id: String,
    name: String,
    account_type: crate::models::AccountType,
    currency: String,
}

impl TryFrom<AccountRow> for Account {
    type Error = LedgerError;

    fn try_from(row: AccountRow) -> Result<Self, Self::Error> {
        Ok(Account {
            id: parse_uuid(&row.id, "account id")?,
            name: row.name,
            account_type: row.account_type,
            currency: row.currency,
        })
    }
}

#[derive(FromRow)]
struct TransactionRow {
    id: String,
    date: NaiveDate,
    description: String,
}

#[derive(FromRow)]
struct PostingRow {
    transaction_id: String,
    account_id: String,
    /// Decimal stored as text; parsed in [`Ledger::transactions`].
    amount: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AccountType;
    use crate::validation::ValidationError;
    use rust_decimal_macros::dec;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn test_ledger() -> Ledger {
        // A single connection keeps the in-memory database alive and ensures
        // all queries see the same database.
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        db::migrate(&pool).await.unwrap();
        Ledger::new(pool)
    }

    fn account(name: &str, account_type: AccountType) -> Account {
        Account {
            id: Uuid::new_v4(),
            name: name.to_string(),
            account_type,
            currency: "USD".to_string(),
        }
    }

    fn transaction(date: NaiveDate, description: &str, postings: Vec<Posting>) -> Transaction {
        Transaction {
            id: Uuid::new_v4(),
            date,
            description: description.to_string(),
            postings,
        }
    }

    #[tokio::test]
    async fn accounts_roundtrip() {
        let ledger = test_ledger().await;
        let checking = account("Checking", AccountType::Asset);
        let visa = account("Visa", AccountType::Liability);

        ledger.insert_account(&checking).await.unwrap();
        ledger.insert_account(&visa).await.unwrap();

        assert_eq!(ledger.accounts().await.unwrap(), vec![checking, visa]);
    }

    #[tokio::test]
    async fn transaction_roundtrip_preserves_exact_amounts() {
        let ledger = test_ledger().await;
        let checking = account("Checking", AccountType::Asset);
        let groceries = account("Groceries", AccountType::Expense);
        ledger.insert_account(&checking).await.unwrap();
        ledger.insert_account(&groceries).await.unwrap();

        let tx = transaction(
            NaiveDate::from_ymd_opt(2026, 9, 19).unwrap(),
            "groceries",
            vec![
                Posting {
                    account_id: groceries.id,
                    amount: dec!(10.10),
                },
                Posting {
                    account_id: checking.id,
                    amount: dec!(-10.10),
                },
            ],
        );
        ledger.insert_transaction(&tx).await.unwrap();

        assert_eq!(ledger.transactions().await.unwrap(), vec![tx]);
    }

    #[tokio::test]
    async fn unbalanced_transaction_is_not_persisted() {
        let ledger = test_ledger().await;
        let checking = account("Checking", AccountType::Asset);
        ledger.insert_account(&checking).await.unwrap();

        let tx = transaction(
            NaiveDate::from_ymd_opt(2026, 9, 19).unwrap(),
            "bad",
            vec![
                Posting {
                    account_id: checking.id,
                    amount: dec!(50.00),
                },
                Posting {
                    account_id: checking.id,
                    amount: dec!(-49.99),
                },
            ],
        );

        match ledger.insert_transaction(&tx).await {
            Err(LedgerError::Validation(ValidationError::Unbalanced(sum))) => {
                assert_eq!(sum, dec!(0.01));
            }
            other => panic!("expected unbalanced validation error, got {other:?}"),
        }
        assert!(ledger.transactions().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn posting_to_missing_account_rolls_back_transaction() {
        let ledger = test_ledger().await;
        // Balanced, but neither referenced account exists: the foreign key
        // violation must abort the whole insert, including the transaction row.
        let tx = transaction(
            NaiveDate::from_ymd_opt(2026, 9, 19).unwrap(),
            "phantom",
            vec![
                Posting {
                    account_id: Uuid::new_v4(),
                    amount: dec!(5.00),
                },
                Posting {
                    account_id: Uuid::new_v4(),
                    amount: dec!(-5.00),
                },
            ],
        );

        assert!(matches!(
            ledger.insert_transaction(&tx).await,
            Err(LedgerError::Database(_))
        ));
        assert!(ledger.transactions().await.unwrap().is_empty());
    }
}
