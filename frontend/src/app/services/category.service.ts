import { HttpClient } from '@angular/common/http';
import { Injectable, inject, signal } from '@angular/core';
import { invoke } from '@tauri-apps/api/core';
import { firstValueFrom } from 'rxjs';

import { Category } from '../models/ledger.model';
import { errorMessage } from './ledger.service';

/**
 * Environment-aware access to transaction categories. Inside the Tauri shell
 * the backend is reached over IPC (`invoke`); in a plain browser it is reached
 * over the Axum HTTP API (`/api/*`, proxied to the backend during `ng serve`).
 */
@Injectable({ providedIn: 'root' })
export class CategoryService {
  private readonly http = inject(HttpClient);
  private readonly isTauri = '__TAURI_INTERNALS__' in window;

  readonly categories = signal<Category[]>([]);
  readonly loading = signal(false);
  readonly error = signal<string | null>(null);

  async refresh(): Promise<void> {
    await this.run(async () => {
      const categories = this.isTauri
        ? await invoke<Category[]>('get_categories')
        : await firstValueFrom(this.http.get<Category[]>('/api/categories'));
      this.categories.set(categories);
    });
  }

  /** Returns whether the category was persisted. */
  async addCategory(category: Category): Promise<boolean> {
    const ok = await this.run(async () => {
      if (this.isTauri) {
        await invoke('insert_category', { category });
      } else {
        await firstValueFrom(this.http.post('/api/categories', category));
      }
      this.categories.update((categories) => [...categories, category]);
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
