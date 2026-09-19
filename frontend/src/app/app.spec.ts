import { signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideNativeDateAdapter } from '@angular/material/core';
import { provideNoopAnimations } from '@angular/platform-browser/animations';

import { App } from './app';
import { CategoryService } from './services/category.service';
import { LedgerService } from './services/ledger.service';

describe('App', () => {
  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [App],
      providers: [
        provideNoopAnimations(),
        provideNativeDateAdapter(),
        {
          provide: LedgerService,
          useValue: {
            accounts: signal([]),
            transactions: signal([]),
            loading: signal(false),
            error: signal(null),
            refresh: () => Promise.resolve(),
          },
        },
        {
          provide: CategoryService,
          useValue: {
            categories: signal([]),
            loading: signal(false),
            error: signal(null),
            refresh: () => Promise.resolve(),
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
