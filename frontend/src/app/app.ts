import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { RouterOutlet } from '@angular/router';

import { AccountTree } from './account-tree/account-tree';
import { Register } from './register/register';
import { LedgerService } from './services/ledger.service';

@Component({
  selector: 'app-root',
  imports: [RouterOutlet, AccountTree, Register],
  templateUrl: './app.html',
  styleUrl: './app.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class App {
  protected readonly ledger = inject(LedgerService);

  constructor() {
    void this.ledger.refresh();
  }
}
