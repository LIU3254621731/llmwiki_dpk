# Repository Guidelines

## Project Structure

LLMWiki is a Tauri 2 desktop app (React/TypeScript frontend + Rust backend) for LLM-powered knowledge base management.

```
src/                    # React frontend (Vite, Tailwind, Radix UI, Zustand)
  components/
    views/              # Dashboard, file explorer, wiki index, graph, review, settings
    layout/             # App shell: IconSidebar, CenterStage, ChatSidebar, StatusBar
    common/             # Shared: MarkdownRenderer, ErrorBoundary, RightContextPanel
    editor/             # CodeMirror-based Markdown editor (edit/preview/split modes)
  stores/               # Zustand stores (useAppStore, useEditorStore, useKBStore, …)
  lib/                  # Types, Tauri command wrappers, Zod schemas
  pages/                # Route-level components
  test/                 # Vitest unit tests
src-tauri/src/          # Rust backend
  commands/             # Tauri #[command] handlers grouped by domain
  core/                 # AppKernel (DI), event bus, task queue
  agents/               # AI pipeline: source_ingest → resolution → relationship → wiki_update
  model/                # LLM gateway, DeepSeek client, ONNX local embeddings
  skills/               # Document processors (PDF, DOCX, HTML, Markdown)
  wiki/                 # Page CRUD, versioning, markdown indexing
  review/               # Diff engine + accept/reject workflow
  graph/                # Knowledge graph via petgraph
  db/                   # SQLite schema + migrations (rusqlite, no ORM)
```

Path alias: `@/` → `src/`. Each knowledge base gets its own `llmwiki.db`.

## Build, Test, and Development Commands

| Command | Purpose |
|---|---|
| `npm run dev` | Vite dev server (port 1420, frontend only) |
| `npm run tauri dev` | Full desktop app (frontend + Rust) |
| `npm run build` | `tsc && vite build` |
| `npm run tauri build` | Production installer |
| `npm run test` | `vitest run` — all unit tests |
| `npm run test:watch` | `vitest` — interactive watch mode |
| `npm run lint` | `tsc --noEmit` — type checking |

Rust code compiles through Tauri. Use `npm run tauri dev` for backend development.

## Coding Style & Naming

- **TypeScript**: Strict mode. PascalCase for components, camelCase for functions/variables.
- **Rust**: Edition 2021. snake_case functions, PascalCase types. Domain-based module layout.
- **CSS**: Tailwind with `class-variance-authority` for variants, `clsx` + `tailwind-merge` for conditional classes.
- **Imports**: Use `@/` path alias (e.g., `import { useAppStore } from '@/stores/useAppStore'`).
- **Lint**: Run `npm run lint` before committing. No auto-formatter configured.

## Testing

- **Framework**: Vitest + `@testing-library/react` + `jsdom`.
- **Location**: `src/test/`, mirroring source modules as `*.test.ts(x)`.
- **Focus**: Store logic, utilities, component rendering. No Rust tests configured.

## Commit & PR Guidelines

- **Commits**: Prefix with version tag (e.g., `v0.2.1: Stability release`).
- **Branches**: Feature branches prefixed with `codex/`.
- **PRs**: Include description, linked issues, and screenshots for UI changes.
- **Do not commit**: `node_modules/`, `dist/`, `src-tauri/target/`, `.env`, `.claude/`.

## Architecture

Frontend ↔ backend via Tauri `invoke()` / `listen()`. AI pipeline: documents → source ingest → entity resolution → relationship discovery → wiki update → human review. See `CLAUDE.md` for detailed architecture docs.
