# Montekristo

## Commands

- `cargo test -p backend` — run backend unit tests.
- `cargo run -p backend --features server` — run the Axum HTTP API (`src/main.rs`
  is behind the `server` feature so `src-tauri` does not pull in web deps).
  - Env: `DATABASE_URL` (default `sqlite://ledger.db`), `BIND_ADDR` (default `127.0.0.1:3000`).
- `cargo check -p app` — typecheck the Tauri shell (`src-tauri`, package `app`).
- `cd frontend && npm start` — Angular dev server on :4200; `proxy.conf.json`
  forwards `/api/*` to `http://localhost:3000`, so run the backend alongside it.
- `cd frontend && npm test` / `npm run build` — Vitest unit tests / production
  build to `dist/frontend/browser` (the path Tauri's `frontendDist` expects).

## API

- `GET|POST /api/accounts`, `GET|POST /api/transactions`
- Errors: `{"error": "..."}` JSON body; 422 for validation failures, 500 otherwise.
- Tauri IPC commands: `get_accounts`, `insert_account`, `get_transactions`, `insert_transaction`.
