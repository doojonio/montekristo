export type AccountType = 'asset' | 'liability' | 'income' | 'expense' | 'equity';

export interface Account {
  id: string;
  name: string;
  account_type: AccountType;
  /** ISO 4217 currency code, e.g. "USD". */
  currency: string;
  /** Parent account id; `null` for roots. */
  parent_id: string | null;
}

export type CategoryType = 'expense' | 'income';

export interface Category {
  id: string;
  name: string;
  category_type: CategoryType;
  /** Optional emoji or icon identifier shown alongside the name. */
  icon: string | null;
}

export interface Posting {
  account_id: string;
  /** Decimal serialized as a string — never a float, to preserve precision. */
  amount: string;
}

export interface Transaction {
  id: string;
  /** Calendar date as "YYYY-MM-DD". */
  date: string;
  description: string;
  /** Optional classification; `null` means uncategorized. */
  category_id: string | null;
  postings: Posting[];
}

/** One row of an account's register, with its running balance. */
export interface LedgerEntry {
  transaction_id: string;
  /** Calendar date as "YYYY-MM-DD". */
  date: string;
  description: string;
  /** Net amount applied to the account (debit positive), as a decimal string. */
  amount: string;
  /** Running balance after this entry, as a decimal string. */
  balance: string;
  /** Counterparty account for two-posting transfers; `null` for splits. */
  transfer_account_id: string | null;
}
