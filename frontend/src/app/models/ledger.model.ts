export type AccountType = 'asset' | 'liability' | 'income' | 'expense' | 'equity';

export interface Account {
  id: string;
  name: string;
  account_type: AccountType;
  /** ISO 4217 currency code, e.g. "USD". */
  currency: string;
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
