import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { MatCardModule } from '@angular/material/card';

import { Category, Transaction } from '../models/ledger.model';
import { CategoryService } from '../services/category.service';
import { LedgerService } from '../services/ledger.service';

@Component({
  selector: 'app-transaction-list',
  imports: [MatCardModule],
  templateUrl: './transaction-list.html',
  styleUrl: './transaction-list.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TransactionList {
  protected readonly ledger = inject(LedgerService);
  protected readonly categories = inject(CategoryService);

  /** Newest first; the backend returns transactions ordered by date. */
  protected readonly sorted = computed(() => [...this.ledger.transactions()].reverse());

  private readonly categoryById = computed(
    () => new Map(this.categories.categories().map((c) => [c.id, c])),
  );
  private readonly accountById = computed(
    () => new Map(this.ledger.accounts().map((a) => [a.id, a])),
  );

  protected categoryOf(transaction: Transaction): Category | undefined {
    return transaction.category_id
      ? this.categoryById().get(transaction.category_id)
      : undefined;
  }

  protected accountName(accountId: string): string {
    return this.accountById().get(accountId)?.name ?? 'Unknown account';
  }

  /** Sum of the debit (positive) postings — the transaction's total outflow. */
  protected total(transaction: Transaction): string {
    return transaction.postings
      .map((p) => p.amount)
      .filter((amount) => !amount.startsWith('-'))
      .reduce(addDecimals, '0');
  }
}

/** Adds two signed decimal strings exactly, without ever using floats. */
function addDecimals(a: string, b: string): string {
  const parts = (value: string) => {
    const negative = value.startsWith('-');
    const [int = '0', frac = ''] = (negative ? value.slice(1) : value).split('.');
    return { negative, int, frac };
  };
  const x = parts(a);
  const y = parts(b);
  const scale = Math.max(x.frac.length, y.frac.length);
  const toScaledInt = (p: { negative: boolean; int: string; frac: string }) =>
    BigInt(p.int + p.frac.padEnd(scale, '0')) * (p.negative ? -1n : 1n);
  const sum = toScaledInt(x) + toScaledInt(y);
  const negative = sum < 0n;
  const digits = (negative ? -sum : sum).toString().padStart(scale + 1, '0');
  const body = scale ? `${digits.slice(0, -scale)}.${digits.slice(-scale)}` : digits;
  return (negative ? '-' : '') + body;
}
