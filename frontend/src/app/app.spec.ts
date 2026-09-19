import { computed, signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideNoopAnimations } from '@angular/platform-browser/animations';

import { App } from './app';
import { LedgerService } from './services/ledger.service';

describe('App', () => {
  beforeEach(async () => {
    const accounts = signal([]);
    await TestBed.configureTestingModule({
      imports: [App],
      providers: [
        provideNoopAnimations(),
        {
          provide: LedgerService,
          useValue: {
            accounts,
            transactions: signal([]),
            loading: signal(false),
            error: signal(null),
            selectedAccountId: signal(null),
            selectedAccount: computed(() => null),
            ledgerEntries: signal([]),
            accountPaths: signal(new Map()),
            refresh: () => Promise.resolve(),
            selectAccount: () => Promise.resolve(),
            addAccount: () => Promise.resolve(true),
            addTransaction: () => Promise.resolve(true),
          },
        },
      ],
    }).compileComponents();
  });

  it('should create the app', () => {
    const fixture = TestBed.createComponent(App);
    const app = fixture.componentInstance;
    expect(app).toBeTruthy();
  });

  it('should render title', async () => {
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    const compiled = fixture.nativeElement as HTMLElement;
    expect(compiled.querySelector('h1')?.textContent).toContain('Montekristo');
  });
});
