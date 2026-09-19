use rust_decimal::Decimal;
use thiserror::Error;

use crate::models::Transaction;

/// Reasons a transaction can fail double-entry validation.
#[derive(Debug, Error, PartialEq)]
pub enum ValidationError {
    #[error("transaction must contain at least two postings")]
    TooFewPostings,
    #[error("transaction is unbalanced: postings sum to {0} instead of zero")]
    Unbalanced(Decimal),
}

impl Transaction {
    /// Enforces the fundamental double-entry rule: a transaction must have at
    /// least two postings and the sum of all posting amounts must equal zero
    /// (i.e. debits equal credits).
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.postings.len() < 2 {
            return Err(ValidationError::TooFewPostings);
        }
        let sum: Decimal = self.postings.iter().map(|p| p.amount).sum();
        if sum != Decimal::ZERO {
            return Err(ValidationError::Unbalanced(sum));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Account, AccountType, Posting};
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;
    use uuid::Uuid;

    fn account(name: &str, account_type: AccountType) -> Account {
        Account {
            id: Uuid::new_v4(),
            name: name.to_string(),
            account_type,
            currency: "USD".to_string(),
        }
    }

    fn transaction(postings: Vec<Posting>) -> Transaction {
        Transaction {
            id: Uuid::new_v4(),
            date: NaiveDate::from_ymd_opt(2026, 9, 19).unwrap(),
            description: "test".to_string(),
            category_id: None,
            postings,
        }
    }

    #[test]
    fn balanced_transaction_is_valid() {
        let checking = account("Checking", AccountType::Asset);
        let groceries = account("Groceries", AccountType::Expense);

        let tx = transaction(vec![
            Posting {
                account_id: groceries.id,
                amount: dec!(50.00),
            },
            Posting {
                account_id: checking.id,
                amount: dec!(-50.00),
            },
        ]);

        assert_eq!(tx.validate(), Ok(()));
    }

    #[test]
    fn balanced_split_transaction_is_valid() {
        let checking = account("Checking", AccountType::Asset);
        let groceries = account("Groceries", AccountType::Expense);
        let dining = account("Dining", AccountType::Expense);

        let tx = transaction(vec![
            Posting {
                account_id: groceries.id,
                amount: dec!(30.25),
            },
            Posting {
                account_id: dining.id,
                amount: dec!(19.75),
            },
            Posting {
                account_id: checking.id,
                amount: dec!(-50.00),
            },
        ]);

        assert_eq!(tx.validate(), Ok(()));
    }

    #[test]
    fn unbalanced_transaction_is_rejected() {
        let checking = account("Checking", AccountType::Asset);
        let groceries = account("Groceries", AccountType::Expense);

        let tx = transaction(vec![
            Posting {
                account_id: groceries.id,
                amount: dec!(50.00),
            },
            Posting {
                account_id: checking.id,
                amount: dec!(-49.99),
            },
        ]);

        assert_eq!(tx.validate(), Err(ValidationError::Unbalanced(dec!(0.01))));
    }

    #[test]
    fn transaction_with_too_few_postings_is_rejected() {
        let checking = account("Checking", AccountType::Asset);

        let empty = transaction(vec![]);
        let single = transaction(vec![Posting {
            account_id: checking.id,
            amount: dec!(0.00),
        }]);

        assert_eq!(empty.validate(), Err(ValidationError::TooFewPostings));
        assert_eq!(single.validate(), Err(ValidationError::TooFewPostings));
    }

    #[test]
    fn transaction_serializes_amounts_without_precision_loss() {
        let checking = account("Checking", AccountType::Asset);
        let savings = account("Savings", AccountType::Asset);

        let tx = transaction(vec![
            Posting {
                account_id: checking.id,
                amount: dec!(10.10),
            },
            Posting {
                account_id: savings.id,
                amount: dec!(-10.10),
            },
        ]);

        let json = serde_json::to_string(&tx).unwrap();
        let roundtrip: Transaction = serde_json::from_str(&json).unwrap();
        assert_eq!(tx, roundtrip);
    }
}
