# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build / Dev Commands

```bash
npm run dev          # Vite dev server on port 1420 (frontend-only, no Tauri)
npm run build        # tsc + vite build (frontend)
npm run tauri dev    # Full Tauri desktop app (frontend + Rust backend)
npm run tauri build  # Production Tauri build
npm run test         # vitest run (frontend unit tests)
npm run test:watch   # vitest in watch mode
npm run typecheck    # tsc --noEmit (also available as `npm run lint`)
```

## Architecture Overview

**LLMWiki** (智维 Wiki) is a Tauri 2 desktop app for LLM-powered knowledge base management. Users upload documents, AI agents extract and organize knowledge into a wiki, and a human reviews/approves changes.

### Stack

- **Frontend**: React 18 + TypeScript, Vite, Tailwind CSS, Radix UI, React Router v6, Zustand, TanStack Query, ReactFlow, CodeMirror 6, motion (Framer Motion fork), chart.js
- **Backend**: Rust (Tauri 2), SQLite (rusqlite/bundled), DeepSeek API for LLM calls, ONNX Runtime (ort) + tokenizers for local embedding
- **Path alias**: `@/` maps to `src/`

### Frontend Layout Architecture

The app shell is `WorkspacePage`, built from three zones:

```text
┌──────────┬────────────────────────────┬──────────┐
│ IconSide │ CenterStage                │ ChatSide │
│ bar      │ (animated view switching)  │ bar      │
│ (60px)   │                            │ (350px)  │
└──────────┴────────────────────────────┴──────────┘
                      │
              StatusBar (bottom)
```

- **IconSidebar** (`src/components/layout/IconSidebar.tsx`) — 60px icon column. Switches the active view via `useAppStore.activeView`. Nav items: dashboard, file_explorer, wiki_index, review_workshop, graph, settings. Bottom icon toggles the chat sidebar. Shows review badge count.
- **CenterStage** (`src/components/layout/CenterStage.tsx`) — Maps `ActiveView` → view component via a static record. Uses `AnimatePresence` for crossfade transitions.
- **ChatSidebar** (`src/components/layout/ChatSidebar.tsx`) — Collapsible AI chat panel (right side).

The old `Sidebar.tsx` (collapsible text sidebar with KB switcher) still exists but is no longer used by the main layout.

### View System

`ActiveView` type (`src/stores/useAppStore.ts`):
`"dashboard" | "file_explorer" | "wiki_index" | "review_workshop" | "graph" | "settings"`

Each view maps to a component in `src/components/views/`:

- `DashboardView` — KB stats, recent activity
- `FileExplorerView` — File tree + source management
- `WikiIndexView` — Wiki page listing and browsing
- `ReviewWorkshopView` — Review queue (diff-based accept/reject)
- `GraphView` — ReactFlow knowledge graph
- `SettingsView` — KB and model configuration

### Frontend Stores (Zustand)

| Store | File | Purpose |
| --- | --- | --- |
| `useAppStore` | `src/stores/useAppStore.ts` | Sidebar visibility, active view, badge counts |
| `useKBStore` | `src/stores/useKBStore.ts` | KB list, current KB selection |
| `useEditorStore` | `src/stores/useEditorStore.ts` | Tab-based document editor: open/close tabs, dirty state, view modes |
| `useReviewStore` | `src/stores/useReviewStore.ts` | Pending review items, accept/reject |
| `useModelStore` | `src/stores/useModelStore.ts` | LLM model configuration |
| `useTaskStore` | `src/stores/useTaskStore.ts` | Import/processing task state |
| `useThemeStore` | `src/stores/useThemeStore.ts` | Dark/light theme |
| `useFileTreeStore` | `src/stores/useFileTreeStore.ts` | File browser tree state |
| `useContextPanelStore` | `src/stores/useContextPanelStore.ts` | Right context panel state |
| `useQuickNavStore` | `src/stores/useQuickNavStore.ts` | Quick navigation shortcuts |

### Frontend Key Patterns

- **Tab management**: `useEditorStore` identifies tabs by `type:path` composite key. Tab types include `"editor" | "wiki" | "file" | "graph" | "dashboard" | "settings"` etc. The Welcome tab (`tab:welcome`) is always pinned and non-closable.
- **View modes** for editor tabs: `"edit" | "preview" | "split"` (CodeMirror editor vs rendered markdown).
- **Component library**: `src/components/common/` holds shared components — `MarkdownRenderer` (with LaTeX support), `ErrorBoundary`, `RightContextPanel`.

### Rust Backend Structure (`src-tauri/src/`)

- `lib.rs` — Tauri builder: registers plugins (dialog, fs, shell, opener), creates `AppKernel` as managed state, registers ~100+ command handlers. Panic hook writes crash logs to `%TEMP%/llmwiki_crash.log`.
- `core/app_kernel.rs` — DI container holding `DatabaseService`, `ConfigService`, `SecretService`, `WorkspaceService`, `EventBus`. On startup, runs recovery checks across all KBs.
- `core/task_queue.rs` — Task lifecycle management with `CancellationToken` support for cooperative cancellation.
- `core/event_bus.rs` — Frontend notification system (Tauri events, e.g. `kb-stats-changed`).
- `db/schema.rs` — SQLite DDL and migrations. Each KB gets its own `llmwiki.db` in its workspace directory.
- `commands/` — Tauri `#[command]` handlers grouped by domain: `workspace`, `config`, `source`, `task`, `review`, `wiki`, `chat_history`, `web_search`, `search`, `graph`, `file_tree`, `source_preview`, `utils`, `token`, `local_fs`.
- `agents/` — AI agent pipeline: `CoordinatorAgent` dispatches tasks through stages: `source_ingest` → `resolution` (entity resolution) → `relationship` (connection discovery) → `wiki_update` (propose page changes). Also includes `health_check` agent.
- `model/` — `ModelGateway` (unified LLM invocation entry point) → `DeepSeekClient` (HTTP client for DeepSeek-compatible API). Also contains local embedding via ONNX Runtime.
- `skills/` — Document processing: `pdf_skill`, `pdf_ocr` (Windows OCR for scanned PDFs), `docx_skill`, `html_skill`, `md_skill`, `txt_skill`, `markitdown_skill`, `web_search_skill`. Orchestrated by `document_processor`.
- `wiki/` — Wiki page CRUD, path management, versioning, markdown indexing, log service.
- `review/` — Review engine with diff generation (`diff_engine`) and batch accept/reject logic (`review_engine`).
- `graph/` — Knowledge graph service (nodes, edges via `petgraph`, health stats, search).
- `search/` — Full-text search and candidate search.
- `schema/` — JSON schema validation and repair (`json_schema_validator`, `json_repair`).
- `recovery/` — Interrupted task detection and recovery, workspace reconciliation.
- `prompts/` — LLM prompt templates and registry.
- `dedup/` — Deduplication service for knowledge items.
- `local_fs/` — Local filesystem operations for file browsing.

### Key Data Flow

1. User creates a KB → workspace directories initialized, `llmwiki.db` created
2. User uploads source files → `source_ingest` task: document parsed, text extracted, AI generates summaries and extracts knowledge items
3. Coordinator dispatches `resolution` → resolves entity references across items
4. Coordinator dispatches `relationship` → discovers connections between entities
5. Coordinator dispatches `wiki_update` → AI proposes new/updated wiki pages
6. Changes enter **Review** queue → human approves/rejects each proposed change (diff view)
7. Approved changes are written to wiki markdown files; knowledge graph is updated
8. **Health Check** monitors for broken links, orphans, and consistency issues

### Frontend-Backend Communication

All IPC goes through Tauri's `invoke`:
```typescript
import { invoke } from "@tauri-apps/api/core";
const kbs = await invoke<any[]>("list_knowledge_bases");
```

Events from backend to frontend via `listen`:
```typescript
import { listen } from "@tauri-apps/api/event";
const unlisten = await listen<any>("kb-stats-changed", (event) => { ... });
```

### Database

SQLite with `rusqlite` (bundled). Each KB's workspace directory contains `llmwiki.db`. Schema path: `src-tauri/src/db/schema.rs`. No ORM — raw SQL with manual migrations.
