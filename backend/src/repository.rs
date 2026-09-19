use std::collections::HashMap;
use std::str::FromStr;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::db;
use crate::error::LedgerError;
use crate::models::{
    Account, Category, LedgerEntry, Posting, Transaction, default_categories,
};

/// SQLite-backed repository for the ledger.
#[derive(Debug, Clone)]
pub struct Ledger {
    pool: SqlitePool,
}

impl Ledger {
    /// Connects to the database at `url` (creating it if necessary), runs
    /// pending migrations, and seeds the default categories. See
    /// [`db::connect`].
    pub async fn connect(url: &str) -> Result<Self, LedgerError> {
        let ledger = Self::new(db::connect(url).await?);
        ledger.seed_default_categories().await?;
        Ok(ledger)
    }

    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Persists a new account.
    ///
    /// The name may be a colon-separated path (e.g.
    /// "Assets:Current Assets:Checking"): missing ancestor accounts are created
    /// with the leaf's type and currency, and the leaf is inserted beneath the
    /// deepest ancestor. A single-segment name is inserted as-is, under
    /// `parent_id` when one is supplied.
    pub async fn insert_account(&self, account: &Account) -> Result<(), LedgerError> {
        let mut account = account.clone();
        let segments = path_segments(&account.name);
        if segments.len() > 1 {
            account.name = segments.last().unwrap().clone();
            account.parent_id = Some(self.ensure_path(&segments, &account).await?);
        }
        insert_account_row(&self.pool, &account).await
    }

    /// Resolves the ancestor chain of `segments` under the root, creating
    /// missing ancestors as clones of `leaf`'s type and currency, and returns
    /// the id of the deepest ancestor (the leaf's parent).
    async fn ensure_path(
        &self,
        segments: &[String],
        leaf: &Account,
    ) -> Result<Uuid, LedgerError> {
        let mut existing: HashMap<(Option<Uuid>, String), Uuid> = self
            .accounts()
            .await?
            .into_iter()
            .map(|a| ((a.parent_id, a.name), a.id))
            .collect();

        let mut parent = None;
        for segment in &segments[..segments.len() - 1] {
            let key = (parent, segment.clone());
            if let Some(id) = existing.get(&key) {
                parent = Some(*id);
                continue;
            }
            let ancestor = Account {
                id: Uuid::new_v4(),
                name: segment.clone(),
                account_type: leaf.account_type,
                currency: leaf.currency.clone(),
                parent_id: parent,
            };
            insert_account_row(&self.pool, &ancestor).await?;
            existing.insert(key, ancestor.id);
            parent = Some(ancestor.id);
        }
        Ok(parent.expect("path has at least one ancestor segment"))
    }

    /// Persists a new category.
    pub async fn insert_category(&self, category: &Category) -> Result<(), LedgerError> {
        sqlx::query("INSERT INTO categories (id, name, category_type, icon) VALUES (?, ?, ?, ?)")
            .bind(category.id.to_string())
            .bind(&category.name)
            .bind(category.category_type)
            .bind(&category.icon)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Inserts the [`default_categories`] when the categories table is empty.
    /// Existing categories (including user-created ones) are left untouched.
    pub async fn seed_default_categories(&self) -> Result<(), LedgerError> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM categories")
            .fetch_one(&self.pool)
            .await?;
        if count > 0 {
            return Ok(());
        }
        let mut tx = self.pool.begin().await?;
        for category in default_categories() {
            sqlx::query("INSERT INTO categories (id, name, category_type, icon) VALUES (?, ?, ?, ?)")
                .bind(category.id.to_string())
                .bind(&category.name)
                .bind(category.category_type)
                .bind(&category.icon)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Returns all categories, ordered by name.
    pub async fn categories(&self) -> Result<Vec<Category>, LedgerError> {
        sqlx::query_as::<_, CategoryRow>(
            "SELECT id, name, category_type, icon FROM categories ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(Category::try_from)
        .collect()
    }

    /// Validates a transaction and persists it together with its postings
    /// atomically, inside a single database transaction.
    pub async fn insert_transaction(&self, transaction: &Transaction) -> Result<(), LedgerError> {
        transaction.validate()?;

        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO transactions (id, date, description, category_id) VALUES (?, ?, ?, ?)",
        )
        .bind(transaction.id.to_string())
        .bind(transaction.date)
        .bind(&transaction.description)
        .bind(transaction.category_id.map(|id| id.to_string()))
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

    /// Returns all accounts — roots and nested — ordered by name. The
    /// hierarchy is expressed through each account's `parent_id`.
    pub async fn accounts(&self) -> Result<Vec<Account>, LedgerError> {
        sqlx::query_as::<_, AccountRow>(
            "SELECT id, name, account_type, currency, parent_id FROM accounts ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(Account::try_from)
        .collect()
    }

    /// Returns the account with `id`, or `None` if it does not exist.
    pub async fn account(&self, id: Uuid) -> Result<Option<Account>, LedgerError> {
        sqlx::query_as::<_, AccountRow>(
            "SELECT id, name, account_type, currency, parent_id FROM accounts WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .map(Account::try_from)
        .transpose()
    }

    /// Returns the register for `account_id`: every transaction touching the
    /// account, oldest first, each carrying the signed amount applied to the
    /// account and the running balance after it.
    pub async fn account_ledger(&self, account_id: Uuid) -> Result<Vec<LedgerEntry>, LedgerError> {
        if self.account(account_id).await?.is_none() {
            return Err(LedgerError::NotFound(format!("account {account_id}")));
        }

        let mut balance = Decimal::ZERO;
        let mut entries = Vec::new();
        for tx in self.transactions().await? {
            let amount: Decimal = tx
                .postings
                .iter()
                .filter(|p| p.account_id == account_id)
                .map(|p| p.amount)
                .sum();
            if !tx.postings.iter().any(|p| p.account_id == account_id) {
                continue;
            }
            balance += amount;
            let transfer_account_id = (tx.postings.len() == 2)
                .then(|| {
                    tx.postings
                        .iter()
                        .find(|p| p.account_id != account_id)
                        .map(|p| p.account_id)
                })
                .flatten();
            entries.push(LedgerEntry {
                transaction_id: tx.id,
                date: tx.date,
                description: tx.description,
                amount,
                balance,
                transfer_account_id,
            });
        }
        Ok(entries)
    }

    /// Returns all transactions with their postings, ordered by date then id.
    /// Postings retain their original insertion order.
    pub async fn transactions(&self) -> Result<Vec<Transaction>, LedgerError> {
        let tx_rows = sqlx::query_as::<_, TransactionRow>(
            "SELECT id, date, description, category_id FROM transactions ORDER BY date, id",
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
                    category_id: row
                        .category_id
                        .map(|cid| parse_uuid(&cid, "transaction category_id"))
                        .transpose()?,
                    postings: postings_by_tx.remove(&id).unwrap_or_default(),
                })
            })
            .collect()
    }
}

fn parse_uuid(value: &str, field: &str) -> Result<Uuid, LedgerError> {
    Uuid::parse_str(value).map_err(|e| LedgerError::CorruptData(format!("{field} {value:?}: {e}")))
}

/// Splits an account path like "Assets : Checking" into trimmed, non-empty
/// segments.
fn path_segments(path: &str) -> Vec<String> {
    path.split(':')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

async fn insert_account_row(pool: &SqlitePool, account: &Account) -> Result<(), LedgerError> {
    sqlx::query(
        "INSERT INTO accounts (id, name, account_type, currency, parent_id) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(account.id.to_string())
    .bind(&account.name)
    .bind(account.account_type)
    .bind(&account.currency)
    .bind(account.parent_id.map(|id| id.to_string()))
    .execute(pool)
    .await?;
    Ok(())
}

/// Ids are stored as hyphenated uuid text; sqlx decodes raw `Uuid` only from
/// 16-byte blobs, so rows carry `String` ids parsed via [`parse_uuid`].
#[derive(FromRow)]
struct AccountRow {
    id: String,
    name: String,
    account_type: crate::models::AccountType,
    currency: String,
    parent_id: Option<String>,
}

impl TryFrom<AccountRow> for Account {
    type Error = LedgerError;

    fn try_from(row: AccountRow) -> Result<Self, Self::Error> {
        Ok(Account {
            id: parse_uuid(&row.id, "account id")?,
            name: row.name,
            account_type: row.account_type,
            currency: row.currency,
            parent_id: row
                .parent_id
                .map(|pid| parse_uuid(&pid, "account parent_id"))
                .transpose()?,
        })
    }
}

#[derive(FromRow)]
struct CategoryRow {
    id: String,
    name: String,
    category_type: crate::models::CategoryType,
    icon: Option<String>,
}

impl TryFrom<CategoryRow> for Category {
    type Error = LedgerError;

    fn try_from(row: CategoryRow) -> Result<Self, Self::Error> {
        Ok(Category {
            id: parse_uuid(&row.id, "category id")?,
            name: row.name,
            category_type: row.category_type,
            icon: row.icon,
        })
    }
}

#[derive(FromRow)]
struct TransactionRow {
    id: String,
    date: NaiveDate,
    description: String,
    category_id: Option<String>,
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
            parent_id: None,
        }
    }

    fn transaction(date: NaiveDate, description: &str, postings: Vec<Posting>) -> Transaction {
        Transaction {
            id: Uuid::new_v4(),
            date,
            description: description.to_string(),
            category_id: None,
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
    async fn colon_path_creates_nested_accounts() {
        let ledger = test_ledger().await;
        let leaf = account("Assets:Current Assets:Checking", AccountType::Asset);
        ledger.insert_account(&leaf).await.unwrap();

        let accounts = ledger.accounts().await.unwrap();
        assert_eq!(accounts.len(), 3);

        let by_name: HashMap<&str, &Account> =
            accounts.iter().map(|a| (a.name.as_str(), a)).collect();
        let assets = by_name["Assets"];
        let current = by_name["Current Assets"];
        let checking = by_name["Checking"];

        assert_eq!(assets.parent_id, None);
        assert_eq!(current.parent_id, Some(assets.id));
        assert_eq!(checking.id, leaf.id);
        assert_eq!(checking.parent_id, Some(current.id));
        // Ancestors inherit the leaf's classification.
        assert!(accounts.iter().all(|a| a.account_type == AccountType::Asset));
    }

    #[tokio::test]
    async fn colon_path_reuses_existing_ancestors() {
        let ledger = test_ledger().await;
        ledger
            .insert_account(&account("Assets", AccountType::Asset))
            .await
            .unwrap();
        ledger
            .insert_account(&account("Assets:Checking", AccountType::Asset))
            .await
            .unwrap();
        ledger
            .insert_account(&account("Assets:Savings", AccountType::Asset))
            .await
            .unwrap();

        let accounts = ledger.accounts().await.unwrap();
        // The shared "Assets" root must not be duplicated.
        assert_eq!(accounts.len(), 3);
        assert_eq!(
            accounts.iter().filter(|a| a.name == "Assets").count(),
            1
        );
    }

    #[tokio::test]
    async fn account_ledger_reports_running_balance_and_transfer() {
        let ledger = test_ledger().await;
        let checking = account("Checking", AccountType::Asset);
        let groceries = account("Groceries", AccountType::Expense);
        let dining = account("Dining", AccountType::Expense);
        for account in [&checking, &groceries, &dining] {
            ledger.insert_account(account).await.unwrap();
        }

        let spend = |date, description, expense: &Account, cents: Decimal| {
            transaction(
                date,
                description,
                vec![
                    Posting {
                        account_id: expense.id,
                        amount: cents,
                    },
                    Posting {
                        account_id: checking.id,
                        amount: -cents,
                    },
                ],
            )
        };
        let first = spend(
            NaiveDate::from_ymd_opt(2026, 9, 17).unwrap(),
            "groceries",
            &groceries,
            dec!(10.10),
        );
        let second = spend(
            NaiveDate::from_ymd_opt(2026, 9, 19).unwrap(),
            "lunch",
            &dining,
            dec!(5.00),
        );
        ledger.insert_transaction(&first).await.unwrap();
        ledger.insert_transaction(&second).await.unwrap();

        let entries = ledger.account_ledger(checking.id).await.unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].transaction_id, first.id);
        assert_eq!(entries[0].amount, dec!(-10.10));
        assert_eq!(entries[0].balance, dec!(-10.10));
        assert_eq!(entries[0].transfer_account_id, Some(groceries.id));
        assert_eq!(entries[1].amount, dec!(-5.00));
        assert_eq!(entries[1].balance, dec!(-15.10));
        assert_eq!(entries[1].transfer_account_id, Some(dining.id));

        // The expense account sees the mirrored, positive leg.
        let entries = ledger.account_ledger(groceries.id).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].amount, dec!(10.10));
        assert_eq!(entries[0].transfer_account_id, Some(checking.id));
    }

    #[tokio::test]
    async fn account_ledger_marks_splits_without_single_transfer() {
        let ledger = test_ledger().await;
        let checking = account("Checking", AccountType::Asset);
        let groceries = account("Groceries", AccountType::Expense);
        let dining = account("Dining", AccountType::Expense);
        for account in [&checking, &groceries, &dining] {
            ledger.insert_account(account).await.unwrap();
        }

        let tx = transaction(
            NaiveDate::from_ymd_opt(2026, 9, 19).unwrap(),
            "split",
            vec![
                Posting {
                    account_id: groceries.id,
                    amount: dec!(30.00),
                },
                Posting {
                    account_id: dining.id,
                    amount: dec!(20.00),
                },
                Posting {
                    account_id: checking.id,
                    amount: dec!(-50.00),
                },
            ],
        );
        ledger.insert_transaction(&tx).await.unwrap();

        let entries = ledger.account_ledger(checking.id).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].amount, dec!(-50.00));
        assert_eq!(entries[0].transfer_account_id, None);
    }

    #[tokio::test]
    async fn account_ledger_rejects_unknown_account() {
        let ledger = test_ledger().await;
        assert!(matches!(
            ledger.account_ledger(Uuid::new_v4()).await,
            Err(LedgerError::NotFound(_))
        ));
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
    async fn categories_roundtrip() {
        let ledger = test_ledger().await;
        let category = Category {
            id: Uuid::new_v4(),
            name: "Groceries".to_string(),
            category_type: crate::models::CategoryType::Expense,
            icon: Some("🛒".to_string()),
        };
        let salary = Category {
            id: Uuid::new_v4(),
            name: "Salary".to_string(),
            category_type: crate::models::CategoryType::Income,
            icon: None,
        };

        ledger.insert_category(&category).await.unwrap();
        ledger.insert_category(&salary).await.unwrap();

        assert_eq!(ledger.categories().await.unwrap(), vec![category, salary]);
    }

    #[tokio::test]
    async fn seed_default_categories_is_idempotent() {
        let ledger = test_ledger().await;

        ledger.seed_default_categories().await.unwrap();
        let seeded = ledger.categories().await.unwrap();
        assert_eq!(seeded.len(), default_categories().len());
        assert!(
            seeded
                .iter()
                .all(|c| c.category_type == crate::models::CategoryType::Expense)
        );

        // A second seed must not duplicate or replace existing categories.
        ledger.seed_default_categories().await.unwrap();
        assert_eq!(ledger.categories().await.unwrap(), seeded);
    }

    #[tokio::test]
    async fn transaction_roundtrip_preserves_category() {
        let ledger = test_ledger().await;
        let checking = account("Checking", AccountType::Asset);
        let groceries_account = account("Groceries", AccountType::Expense);
        ledger.insert_account(&checking).await.unwrap();
        ledger.insert_account(&groceries_account).await.unwrap();
        let category = Category {
            id: Uuid::new_v4(),
            name: "Groceries".to_string(),
            category_type: crate::models::CategoryType::Expense,
            icon: None,
        };
        ledger.insert_category(&category).await.unwrap();

        let mut tx = transaction(
            NaiveDate::from_ymd_opt(2026, 9, 19).unwrap(),
            "groceries",
            vec![
                Posting {
                    account_id: groceries_account.id,
                    amount: dec!(10.10),
                },
                Posting {
                    account_id: checking.id,
                    amount: dec!(-10.10),
                },
            ],
        );
        tx.category_id = Some(category.id);
        ledger.insert_transaction(&tx).await.unwrap();

        assert_eq!(ledger.transactions().await.unwrap(), vec![tx]);
    }

    #[tokio::test]
    async fn transaction_with_missing_category_is_rejected() {
        let ledger = test_ledger().await;
        let checking = account("Checking", AccountType::Asset);
        let groceries = account("Groceries", AccountType::Expense);
        ledger.insert_account(&checking).await.unwrap();
        ledger.insert_account(&groceries).await.unwrap();

        let mut tx = transaction(
            NaiveDate::from_ymd_opt(2026, 9, 19).unwrap(),
            "phantom category",
            vec![
                Posting {
                    account_id: groceries.id,
                    amount: dec!(5.00),
                },
                Posting {
                    account_id: checking.id,
                    amount: dec!(-5.00),
                },
            ],
        );
        tx.category_id = Some(Uuid::new_v4());

        assert!(matches!(
            ledger.insert_transaction(&tx).await,
            Err(LedgerError::Database(_))
        ));
        assert!(ledger.transactions().await.unwrap().is_empty());
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
