---
trigger: always_on
---
# Project Rules & Architecture

## Tech Stack
- **Backend:** Rust (Workspace members: `backend`, `src-tauri`), SQLite (`sqlx`), Fixed-point math (`rust_decimal`).
- **Frontend:** Angular (Standalone components, Signals, Reactive Forms, Tailwind CSS).
- **Desktop Wrapper:** Tauri v2 (`src-tauri`).

## Engineering Principles
- **Strict Financial Math:** NEVER use floating-point numbers (`f32`/`f64`) for currency calculations in Rust. Always use `rust_decimal`.
- **Monorepo Discipline:** Respect workspace boundaries. Backend logic lives in `backend/`, desktop configurations in `src-tauri/`, and UI in `frontend/`.
- **Angular Standards:** Always follow the rules defined in `guidelines.md` (e.g., use Signals, `inject()`, native control flow `@if`/`@for`, and NEVER set `standalone: true` since it's default in Angular v20+).
- **Context Efficiency:** Keep file modifications concise and modular. Avoid generating massive monolithic files.