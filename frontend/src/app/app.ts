import { ChangeDetectionStrategy, Component } from '@angular/core';
import { RouterOutlet } from '@angular/router';

import { QuickTransaction } from './quick-transaction/quick-transaction';
import { TransactionList } from './transaction-list/transaction-list';

@Component({
  selector: 'app-root',
  imports: [RouterOutlet, QuickTransaction, TransactionList],
  templateUrl: './app.html',
  styleUrl: './app.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class App {}
