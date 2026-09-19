import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { FormControl, FormGroup, ReactiveFormsModule, Validators } from '@angular/forms';

import { Account, AccountType } from '../models/ledger.model';
import { LedgerService } from '../services/ledger.service';
import { addDecimals, formatAmount } from '../utils/decimal';

/** A node in the account hierarchy, with its own and recursive totals. */
interface AccountNode {
  account: Account;
  depth: number;
  children: AccountNode[];
  /** Sum of this account's own postings. */
  balance: string;
  /** `balance` plus the totals of all descendants. */
  total: string;
}

const ACCOUNT_TYPES: { value: AccountType; label: string }[] = [
  { value: 'asset', label: 'Asset' },
  { value: 'liability', label: 'Liability' },
  { value: 'income', label: 'Income' },
  { value: 'expense', label: 'Expense' },
  { value: 'equity', label: 'Equity' },
];

@Component({
  selector: 'app-account-tree',
  imports: [ReactiveFormsModule],
  templateUrl: './account-tree.html',
  styleUrl: './account-tree.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AccountTree {
  protected readonly ledger = inject(LedgerService);
  protected readonly types = ACCOUNT_TYPES;
  protected readonly formatAmount = formatAmount;

  /** Ids of collapsed parents; everything starts expanded. */
  protected readonly collapsed = signal<ReadonlySet<string>>(new Set());
  protected readonly showNewAccount = signal(false);

  protected readonly newAccountForm = new FormGroup({
    path: new FormControl('', { nonNullable: true, validators: [Validators.required] }),
    type: new FormControl<AccountType>('asset', { nonNullable: true }),
    currency: new FormControl('USD', {
      nonNullable: true,
      validators: [Validators.required, Validators.pattern(/^[A-Za-z]{3}$/)],
    }),
  });

  /** Signed sum of each account's own postings across all transactions. */
  private readonly balances = computed(() => {
    const sums = new Map<string, string>();
    for (const tx of this.ledger.transactions()) {
      for (const p of tx.postings) {
        sums.set(p.account_id, addDecimals(sums.get(p.account_id) ?? '0', p.amount));
      }
    }
    return sums;
  });

  /** Root-level nodes; children nest recursively beneath their parents. */
  private readonly roots = computed<AccountNode[]>(() => {
    const ids = new Set(this.ledger.accounts().map((a) => a.id));
    const byParent = new Map<string | null, Account[]>();
    for (const account of this.ledger.accounts()) {
      // An account whose parent is missing (orphan) is treated as a root.
      const parent = account.parent_id && ids.has(account.parent_id) ? account.parent_id : null;
      byParent.set(parent, [...(byParent.get(parent) ?? []), account]);
    }
    const build = (parentId: string | null, depth: number): AccountNode[] =>
      [...(byParent.get(parentId) ?? [])]
        .sort((a, b) => a.name.localeCompare(b.name))
        .map((account) => {
          const children = build(account.id, depth + 1);
          const balance = this.balances().get(account.id) ?? '0';
          const total = children.reduce((sum, c) => addDecimals(sum, c.total), balance);
          return { account, depth, children, balance, total };
        });
    return build(null, 0);
  });

  /** The tree flattened to visible rows, honoring the collapsed set. */
  protected readonly rows = computed(() => {
    const flat: AccountNode[] = [];
    const walk = (nodes: AccountNode[]) => {
      for (const node of nodes) {
        flat.push(node);
        if (!this.collapsed().has(node.account.id)) {
          walk(node.children);
        }
      }
    };
    walk(this.roots());
    return flat;
  });

  protected toggle(id: string): void {
    this.collapsed.update((set) => {
      const next = new Set(set);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  }

  protected select(id: string): void {
    void this.ledger.selectAccount(id);
  }

  /**
   * Creates an account from a colon-separated path. The backend resolves the
   * hierarchy, creating any missing ancestors.
   */
  protected async createAccount(): Promise<void> {
    if (this.newAccountForm.invalid) {
      this.newAccountForm.markAllAsTouched();
      return;
    }
    const { path, type, currency } = this.newAccountForm.getRawValue();
    const ok = await this.ledger.addAccount({
      id: crypto.randomUUID(),
      name: path.trim(),
      account_type: type,
      currency: currency.trim().toUpperCase(),
      parent_id: null,
    });
    if (ok) {
      this.newAccountForm.reset({ path: '', type, currency: currency.trim().toUpperCase() });
      this.showNewAccount.set(false);
    }
  }
}
