use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The fundamental accounting classification of an account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum AccountType {
    Asset,
    Liability,
    Income,
    Expense,
    Equity,
}

/// A named ledger account holding balances in a single currency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub id: Uuid,
    pub name: String,
    pub account_type: AccountType,
    /// ISO 4217 currency code, e.g. "USD".
    pub currency: String,
}

/// Whether a category classifies outflows or inflows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum CategoryType {
    Expense,
    Income,
}

/// A user-facing label used to classify transactions (e.g. "Groceries").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Category {
    pub id: Uuid,
    pub name: String,
    pub category_type: CategoryType,
    /// Optional emoji or icon identifier shown alongside the name.
    pub icon: Option<String>,
}

/// The categories seeded into a fresh database.
pub fn default_categories() -> Vec<Category> {
    [
        ("Groceries", "🛒"),
        ("Gifts", "🎁"),
        ("Dining", "🍽️"),
        ("Bills", "🧾"),
        ("Transport", "🚗"),
    ]
    .into_iter()
    .map(|(name, icon)| Category {
        id: Uuid::new_v4(),
        name: name.to_string(),
        category_type: CategoryType::Expense,
        icon: Some(icon.to_string()),
    })
    .collect()
}

/// A single debit or credit against an account within a transaction.
///
/// Sign convention: debits are positive, credits are negative.
/// The amounts of all postings in a transaction must sum to zero.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Posting {
    pub account_id: Uuid,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
}

/// A double-entry transaction: a dated set of postings that must balance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Uuid,
    pub date: NaiveDate,
    pub description: String,
    /// Optional classification; `None` means uncategorized.
    pub category_id: Option<Uuid>,
    pub postings: Vec<Posting>,
}
