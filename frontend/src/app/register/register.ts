import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  ViewChild,
  computed,
  inject,
  signal,
} from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { FormControl, FormGroup, ReactiveFormsModule } from '@angular/forms';
import {
  MatAutocompleteModule,
  MatAutocompleteTrigger,
} from '@angular/material/autocomplete';

import { LedgerEntry, Transaction } from '../models/ledger.model';
import { LedgerService } from '../services/ledger.service';
import {
  absDecimal,
  addDecimals,
  decimalSign,
  formatAmount,
  negateDecimal,
  parseAmountInput,
} from '../utils/decimal';

/** Local calendar date as "YYYY-MM-DD" (toISOString would shift by timezone). */
function todayIso(): string {
  const now = new Date();
  const month = String(now.getMonth() + 1).padStart(2, '0');
  const day = String(now.getDate()).padStart(2, '0');
  return `${now.getFullYear()}-${month}-${day}`;
}

@Component({
  selector: 'app-register',
  imports: [ReactiveFormsModule, MatAutocompleteModule],
  templateUrl: './register.html',
  styleUrl: './register.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class Register {
  protected readonly ledger = inject(LedgerService);
  protected readonly formatAmount = formatAmount;
  protected readonly entryError = signal<string | null>(null);

  @ViewChild('dateInput') private dateInput?: ElementRef<HTMLInputElement>;

  /** The sticky blank row at the bottom of the register. */
  protected readonly entryForm = new FormGroup({
    date: new FormControl(todayIso(), { nonNullable: true }),
    description: new FormControl('', { nonNullable: true }),
    /** Transfer target expressed as a full account path. */
    transfer: new FormControl('', { nonNullable: true }),
    deposit: new FormControl('', { nonNullable: true }),
    withdrawal: new FormControl('', { nonNullable: true }),
  });

  private readonly accountByPath = computed(
    () => new Map([...this.ledger.accountPaths()].map(([id, path]) => [path, id])),
  );

  /** Distinct past descriptions on the open register, newest first. */
  private readonly historyDescriptions = computed(() => {
    const selected = this.ledger.selectedAccountId();
    if (!selected) {
      return [];
    }
    const seen = new Set<string>();
    const descriptions: string[] = [];
    for (const tx of [...this.ledger.transactions()].reverse()) {
      const description = tx.description.trim();
      if (!description || seen.has(description)) {
        continue;
      }
      if (tx.postings.some((p) => p.account_id === selected)) {
        seen.add(description);
        descriptions.push(description);
      }
    }
    return descriptions;
  });

  private readonly descriptionQuery = toSignal(
    this.entryForm.controls.description.valueChanges,
    { initialValue: '' },
  );
  protected readonly descriptionOptions = computed(() => {
    const query = this.descriptionQuery().trim().toLowerCase();
    return this.historyDescriptions()
      .filter((d) => d.toLowerCase().includes(query))
      .slice(0, 8);
  });

  /** Selectable transfer targets: every account path except the open register. */
  private readonly transferPaths = computed(() => {
    const selected = this.ledger.selectedAccountId();
    return [...this.ledger.accountPaths()]
      .filter(([id]) => id !== selected)
      .map(([, path]) => path)
      .sort((a, b) => a.localeCompare(b));
  });
  private readonly transferQuery = toSignal(this.entryForm.controls.transfer.valueChanges, {
    initialValue: '',
  });
  protected readonly transferOptions = computed(() => {
    const query = this.transferQuery().trim().toLowerCase();
    return this.transferPaths()
      .filter((p) => p.toLowerCase().includes(query))
      .slice(0, 10);
  });

  protected accountPath(id: string): string {
    return this.ledger.accountPaths().get(id) ?? 'Unknown account';
  }

  protected transferOf(entry: LedgerEntry): string {
    if (!entry.transfer_account_id) {
      return '— Split —';
    }
    return this.accountPath(entry.transfer_account_id);
  }

  protected depositOf(entry: LedgerEntry): string {
    return decimalSign(entry.amount) > 0 ? formatAmount(entry.amount) : '';
  }

  protected withdrawalOf(entry: LedgerEntry): string {
    return decimalSign(entry.amount) < 0 ? formatAmount(absDecimal(entry.amount)) : '';
  }

  /** Escape clears the blank row, unless an autocomplete panel is open. */
  protected onEscape(trigger?: MatAutocompleteTrigger): void {
    if (trigger?.panelOpen) {
      return; // the panel consumes Escape to close itself
    }
    this.resetEntry();
  }

  /** Typing a deposit clears the withdrawal cell, and vice versa. */
  protected onDepositInput(): void {
    if (this.entryForm.controls.deposit.value.trim()) {
      this.entryForm.controls.withdrawal.setValue('', { emitEvent: false });
    }
  }

  protected onWithdrawalInput(): void {
    if (this.entryForm.controls.withdrawal.value.trim()) {
      this.entryForm.controls.deposit.setValue('', { emitEvent: false });
    }
  }

  /**
   * Pre-fills the transfer account (and amount, when the cells are still
   * empty) from the most recent transaction on this register whose
   * description matches — the GnuCash-style quick-fill.
   */
  protected applyDescriptionHistory(explicit?: string): void {
    const selected = this.ledger.selectedAccountId();
    const description = (explicit ?? this.entryForm.controls.description.value).trim();
    if (!selected || !description) {
      return;
    }
    const match = [...this.ledger.transactions()]
      .reverse()
      .find(
        (tx) =>
          tx.description.trim() === description &&
          tx.postings.some((p) => p.account_id === selected),
      );
    if (!match) {
      return;
    }

    const controls = this.entryForm.controls;
    const others = match.postings.filter((p) => p.account_id !== selected);
    if (!controls.transfer.value.trim() && others.length === 1) {
      const path = this.ledger.accountPaths().get(others[0].account_id);
      if (path) {
        controls.transfer.setValue(path);
      }
    }
    if (!controls.deposit.value.trim() && !controls.withdrawal.value.trim()) {
      const mine = match.postings
        .filter((p) => p.account_id === selected)
        .reduce((sum, p) => addDecimals(sum, p.amount), '0');
      if (decimalSign(mine) > 0) {
        controls.deposit.setValue(mine);
      } else if (decimalSign(mine) < 0) {
        controls.withdrawal.setValue(absDecimal(mine));
      }
    }
  }

  protected resetEntry(): void {
    this.entryForm.reset({
      date: todayIso(),
      description: '',
      transfer: '',
      deposit: '',
      withdrawal: '',
    });
    this.entryError.set(null);
  }

  /**
   * Commits the blank row as a two-posting transaction and leaves a fresh
   * blank row focused — the register's Enter keybinding.
   */
  protected async commit(): Promise<void> {
    const selected = this.ledger.selectedAccountId();
    if (!selected) {
      return;
    }
    const value = this.entryForm.getRawValue();
    const description = value.description.trim();
    const depositRaw = value.deposit.trim();
    const withdrawalRaw = value.withdrawal.trim();
    const deposit = depositRaw ? parseAmountInput(depositRaw) : null;
    const withdrawal = withdrawalRaw ? parseAmountInput(withdrawalRaw) : null;
    const transferId = this.accountByPath().get(value.transfer.trim());

    if (!value.date) {
      return this.fail('Enter a date');
    }
    if (!description) {
      return this.fail('Enter a description');
    }
    if (!transferId) {
      return this.fail(
        value.transfer.trim() ? 'Unknown transfer account' : 'Choose a transfer account',
      );
    }
    if (transferId === selected) {
      return this.fail('Transfer account must differ from the open register');
    }
    if (!depositRaw && !withdrawalRaw) {
      return this.fail('Enter a deposit or withdrawal amount');
    }
    if (depositRaw && !deposit) {
      return this.fail('Invalid deposit amount');
    }
    if (withdrawalRaw && !withdrawal) {
      return this.fail('Invalid withdrawal amount');
    }

    // Deposit: the register account is debited and the transfer credited.
    const amount = deposit ?? `-${withdrawal!}`;
    const transaction: Transaction = {
      id: crypto.randomUUID(),
      date: value.date,
      description,
      category_id: null,
      postings: [
        { account_id: selected, amount },
        { account_id: transferId, amount: negateDecimal(amount) },
      ],
    };
    if (await this.ledger.addTransaction(transaction)) {
      this.resetEntry();
      this.dateInput?.nativeElement.focus();
    } else {
      this.entryError.set(this.ledger.error() ?? 'Could not record the transaction');
    }
  }

  private fail(message: string): void {
    this.entryError.set(message);
  }
}
