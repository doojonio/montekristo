import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import {
  AbstractControl,
  FormControl,
  FormGroup,
  ReactiveFormsModule,
  ValidationErrors,
  ValidatorFn,
  Validators,
} from '@angular/forms';
import { MatButtonModule } from '@angular/material/button';
import { MatCardModule } from '@angular/material/card';
import { MatDatepickerModule } from '@angular/material/datepicker';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import { MatSelectModule } from '@angular/material/select';
import { MatSnackBar, MatSnackBarModule } from '@angular/material/snack-bar';

import { Transaction } from '../models/ledger.model';
import { LedgerService } from '../services/ledger.service';

/** Accepts a positive decimal string — validated without ever parsing a float. */
const amountValidator: ValidatorFn = (
  control: AbstractControl<string>,
): ValidationErrors | null => {
  const value = control.value;
  if (!value) {
    return null; // `required` reports empty values.
  }
  if (!/^\d+(\.\d+)?$/.test(value)) {
    return { amount: 'Enter a positive amount, e.g. 12.50' };
  }
  if (!/[1-9]/.test(value)) {
    return { amount: 'Amount must be greater than zero' };
  }
  return null;
};

/** The debit and credit legs must target different accounts. */
const differentAccountsValidator: ValidatorFn = (
  group: AbstractControl,
): ValidationErrors | null => {
  const debit = group.get('debitAccountId')?.value;
  const credit = group.get('creditAccountId')?.value;
  return debit && credit && debit === credit ? { sameAccount: true } : null;
};

@Component({
  selector: 'app-quick-transaction',
  imports: [
    ReactiveFormsModule,
    MatButtonModule,
    MatCardModule,
    MatDatepickerModule,
    MatFormFieldModule,
    MatInputModule,
    MatSelectModule,
    MatSnackBarModule,
  ],
  templateUrl: './quick-transaction.html',
  styleUrl: './quick-transaction.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class QuickTransaction {
  protected readonly ledger = inject(LedgerService);
  private readonly snackBar = inject(MatSnackBar);

  protected readonly form = new FormGroup(
    {
      date: new FormControl(new Date(), {
        nonNullable: true,
        validators: [Validators.required],
      }),
      description: new FormControl('', {
        nonNullable: true,
        validators: [Validators.required, Validators.maxLength(200)],
      }),
      debitAccountId: new FormControl('', {
        nonNullable: true,
        validators: [Validators.required],
      }),
      creditAccountId: new FormControl('', {
        nonNullable: true,
        validators: [Validators.required],
      }),
      amount: new FormControl('', {
        nonNullable: true,
        validators: [Validators.required, Validators.maxLength(30), amountValidator],
      }),
    },
    { validators: differentAccountsValidator },
  );

  constructor() {
    void this.ledger.refresh();
  }

  protected async submit(): Promise<void> {
    if (this.form.invalid) {
      this.form.markAllAsTouched();
      return;
    }
    const { date, description, debitAccountId, creditAccountId, amount } =
      this.form.getRawValue();
    const transaction: Transaction = {
      id: crypto.randomUUID(),
      date: toIsoDate(date),
      description: description.trim(),
      postings: [
        { account_id: debitAccountId, amount },
        { account_id: creditAccountId, amount: `-${amount}` },
      ],
    };
    if (await this.ledger.addTransaction(transaction)) {
      this.snackBar.open('Transaction recorded', undefined, { duration: 3000 });
      this.form.reset({ date: new Date() });
    }
  }
}

/** Local calendar date as "YYYY-MM-DD" (toISOString would shift by timezone). */
function toIsoDate(date: Date): string {
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${date.getFullYear()}-${month}-${day}`;
}
