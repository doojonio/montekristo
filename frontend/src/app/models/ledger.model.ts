export type AccountType = 'asset' | 'liability' | 'income' | 'expense' | 'equity';

export interface Account {
  id: string;
  name: string;
  account_type: AccountType;
  /** ISO 4217 currency code, e.g. "USD". */
  currency: string;
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
  postings: Posting[];
}
