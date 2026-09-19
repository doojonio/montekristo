import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { Injectable, inject, signal } from '@angular/core';
import { invoke } from '@tauri-apps/api/core';
import { firstValueFrom } from 'rxjs';

import { Account, Transaction } from '../models/ledger.model';

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

  async refresh(): Promise<void> {
    await this.run(async () => {
      const [accounts, transactions] = this.isTauri
        ? await Promise.all([
            invoke<Account[]>('get_accounts'),
            invoke<Transaction[]>('get_transactions'),
          ])
        : await Promise.all([
            firstValueFrom(this.http.get<Account[]>('/api/accounts')),
            firstValueFrom(this.http.get<Transaction[]>('/api/transactions')),
          ]);
      this.accounts.set(accounts);
      this.transactions.set(transactions);
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
      this.accounts.update((accounts) => [...accounts, account]);
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
      return true;
    });
    return ok ?? false;
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

export function errorMessage(e: unknown): string {
  // The HTTP API renders failures as `{ "error": "..." }` JSON bodies.
  if (e instanceof HttpErrorResponse) {
    const body = e.error as { error?: unknown } | null;
    return typeof body?.error === 'string' ? body.error : e.message;
  }
  return e instanceof Error ? e.message : String(e);
}
