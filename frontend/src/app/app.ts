import { ChangeDetectionStrategy, Component } from '@angular/core';
import { RouterOutlet } from '@angular/router';

import { QuickTransaction } from './quick-transaction/quick-transaction';

@Component({
  selector: 'app-root',
  imports: [RouterOutlet, QuickTransaction],
  templateUrl: './app.html',
  styleUrl: './app.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class App {}
