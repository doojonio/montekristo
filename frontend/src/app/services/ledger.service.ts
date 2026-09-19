import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { Injectable, computed, inject, signal } from '@angular/core';
import { invoke } from '@tauri-apps/api/core';
import { firstValueFrom } from 'rxjs';

import { Account, LedgerEntry, Transaction } from '../models/ledger.model';

/**
 * Environment-aware access to the ledger. Inside the Tauri shell the backend
 * is reached over IPC (`invoke`); in a plain browser it is reached over the
 * Axum HTTP API (`/api/*`, proxied to the backend during `ng serve`).
 */
@Injectable({ providedIn: 'root' })
export class LedgerService {
  private readonly http = inject(HttpClient);
  private readonly isTauri = '__TAURI_INTERNALS__' in window;

  readonly accounts = signal<Account[]>([]);
  readonly transactions = signal<Transaction[]>([]);
  readonly loading = signal(false);
  readonly error = signal<string | null>(null);

  /** The account whose register is open in the right-hand pane. */
  readonly selectedAccountId = signal<string | null>(null);
  /** Register rows for the selected account, oldest first. */
  readonly ledgerEntries = signal<LedgerEntry[]>([]);
  readonly selectedAccount = computed(
    () => this.accounts().find((a) => a.id === this.selectedAccountId()) ?? null,
  );

  /** Full colon-separated path of every account, e.g. "Assets:Checking". */
  readonly accountPaths = computed(() => {
    const byId = new Map(this.accounts().map((a) => [a.id, a]));
    const paths = new Map<string, string>();
    const pathOf = (account: Account): string => {
      const cached = paths.get(account.id);
      if (cached !== undefined) {
        return cached;
      }
      const parent = account.parent_id ? byId.get(account.parent_id) : undefined;
      const path = parent ? `${pathOf(parent)}:${account.name}` : account.name;
      paths.set(account.id, path);
      return path;
    };
    for (const account of this.accounts()) {
      pathOf(account);
    }
    return paths;
  });

  async refresh(): Promise<void> {
    await this.run(async () => {
      const [accounts, transactions] = await Promise.all([
        this.fetchAccounts(),
        this.fetchTransactions(),
      ]);
      this.accounts.set(accounts);
      this.transactions.set(transactions);
      await this.ensureSelection(accounts);
    });
  }

  /** Opens the register for `id` in the right-hand pane. */
  async selectAccount(id: string): Promise<void> {
    if (id === this.selectedAccountId()) {
      return;
    }
    this.selectedAccountId.set(id);
    await this.run(async () => {
      this.ledgerEntries.set(await this.fetchLedger(id));
    });
  }

  /** Returns whether the account was persisted. */
  async addAccount(account: Account): Promise<boolean> {
    const ok = await this.run(async () => {
      if (this.isTauri) {
        await invoke('insert_account', { account });
      } else {
        await firstValueFrom(this.http.post('/api/accounts', account));
      }
      // Colon-path inserts may create ancestor accounts; re-fetch the list
      // rather than appending the leaf locally.
      const accounts = await this.fetchAccounts();
      this.accounts.set(accounts);
      await this.ensureSelection(accounts);
      return true;
    });
    return ok ?? false;
  }

  /** Returns whether the transaction was persisted. */
  async addTransaction(transaction: Transaction): Promise<boolean> {
    const ok = await this.run(async () => {
      if (this.isTauri) {
        await invoke('insert_transaction', { transaction });
      } else {
        await firstValueFrom(this.http.post('/api/transactions', transaction));
      }
      this.transactions.update((transactions) => [...transactions, transaction]);
      const selected = this.selectedAccountId();
      if (selected && transaction.postings.some((p) => p.account_id === selected)) {
        this.ledgerEntries.set(await this.fetchLedger(selected));
      }
      return true;
    });
    return ok ?? false;
  }

  /** Keeps the current selection if it still exists, else picks a default. */
  private async ensureSelection(accounts: Account[]): Promise<void> {
    const current = this.selectedAccountId();
    const target =
      current && accounts.some((a) => a.id === current) ? current : defaultAccountId(accounts);
    this.selectedAccountId.set(target);
    this.ledgerEntries.set(target ? await this.fetchLedger(target) : []);
  }

  private fetchAccounts(): Promise<Account[]> {
    return this.isTauri
      ? invoke<Account[]>('get_accounts')
      : firstValueFrom(this.http.get<Account[]>('/api/accounts'));
  }

  private fetchTransactions(): Promise<Transaction[]> {
    return this.isTauri
      ? invoke<Transaction[]>('get_transactions')
      : firstValueFrom(this.http.get<Transaction[]>('/api/transactions'));
  }

  private fetchLedger(accountId: string): Promise<LedgerEntry[]> {
    return this.isTauri
      ? invoke<LedgerEntry[]>('get_account_ledger', { accountId })
      : firstValueFrom(this.http.get<LedgerEntry[]>(`/api/accounts/${accountId}/ledger`));
  }

  private async run<T>(operation: () => Promise<T>): Promise<T | undefined> {
    this.loading.set(true);
    this.error.set(null);
    try {
      return await operation();
    } catch (e) {
      this.error.set(errorMessage(e));
      return undefined;
    } finally {
      this.loading.set(false);
    }
  }
}

/** The first leaf account, or the first account overall. */
function defaultAccountId(accounts: Account[]): string | null {
  const parentIds = new Set(
    accounts.map((a) => a.parent_id).filter((id): id is string => id !== null),
  );
  return (accounts.find((a) => !parentIds.has(a.id)) ?? accounts[0])?.id ?? null;
}

export function errorMessage(e: unknown): string {
  // The HTTP API renders failures as `{ "error": "..." }` JSON bodies.
  if (e instanceof HttpErrorResponse) {
    const body = e.error as { error?: unknown } | null;
    return typeof body?.error === 'string' ? body.error : e.message;
  }
  return e instanceof Error ? e.message : String(e);
}
