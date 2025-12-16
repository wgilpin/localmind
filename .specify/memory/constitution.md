<!--
Sync Impact Report
==================
Version change: 1.0.0 → 1.1.0
Modified principles: I (Privacy), II (Performance), IV (Automation), V (Developer Quality)
Added sections: Principle VI (Python Development Standards)
Removed concepts: Ollama, LM Studio, chat features, streaming responses (app is now embeddings-only for RAG indexing and search)
Templates requiring updates:
  ✅ .specify/templates/plan-template.md - reviewed, compatible
  ✅ .specify/templates/spec-template.md - reviewed, compatible
  ✅ .specify/templates/tasks-template.md - reviewed, compatible
Follow-up TODOs: None
-->

# LocalMind-rs Constitution

## Core Principles

### I. Privacy & Offline-First Architecture

**Non-Negotiable Rules:**

- All data processing MUST occur locally on the user's device
- SQLite database MUST use bundled rusqlite with no external dependencies
- Data MUST be encrypted at rest in `~/.localmind/` (Windows: `%APPDATA%/localmind/`)
- Zero cloud dependencies for core functionality
- Network requests ONLY to localhost:8000 (Python embedding server) - no external or remote requests
- No telemetry, analytics, or external data transmission without explicit user consent
- Zero-knowledge architecture: relay servers (if implemented) MUST NOT have access to 
  unencrypted user data

**Rationale:** Users trust LocalMind with sensitive research and personal notes. Privacy is
not a feature but the foundation. Offline-first ensures reliability independent of network
conditions and protects against service disruptions.

### II. Performance & Native Experience

**Non-Negotiable Rules:**

- Search responses MUST target <100ms median latency
- Memory footprint MUST target <50MB for core application (excludes Python embedding server)
- Single executable deployment with auto-managed Python server (no manual setup)
- Rust async runtime (Tokio) for all I/O operations
- Cross-platform compatibility: Windows, macOS, Linux
- Native window decorations and OS integration via Tauri

**Rationale:** Desktop applications compete with instant web search. Sub-100ms responses
maintain user flow. Rust's memory safety prevents crashes that erode trust. Single
executable reduces friction for non-technical users.

### III. Modern UI/UX Excellence

**Non-Negotiable Rules:**

- Svelte 5 runes (`$state`, `$effect`, `$derived`, `$props`) MUST be used for reactivity
- UI MUST be accessible per WCAG 2.2 AA standards
- All user-facing text MUST be clear and jargon-free
- Error messages MUST be actionable (tell user what to do, not just what failed)
- Loading states MUST be shown for operations >200ms
- Keyboard navigation MUST be fully supported
- Dark/light mode support (if implemented) MUST respect OS preferences

**Rationale:** Users interact with the UI hundreds of times daily. Accessible design
benefits all users, not just those with disabilities. Svelte 5 runes provide predictable
reactivity without framework magic.

### IV. Intelligent Automation with User Control

**Non-Negotiable Rules:**

- Bookmark monitoring MUST be opt-in with clear explanation
- Users MUST be able to exclude folders and domains before ingestion starts
- All automated actions MUST be visible in UI (e.g., "Monitoring 127 bookmarks")
- Search MUST degrade gracefully if embedding server unavailable (show cached results or clear error)
- No background data collection without active user session
- Users MUST be able to export all data in standard formats (JSON, CSV)
- Configuration MUST be editable via UI (no manual file editing required)

**Rationale:** Automation saves time, but invisible automation erodes trust. Users must
remain in control of their data. Graceful degradation ensures core search works even if
embedding server setup fails.

### V. Developer Quality & Maintainability

**Non-Negotiable Rules:**

- All code MUST pass `cargo clippy` with no warnings
- All code MUST be formatted with `cargo fmt` before commit
- All new modules with business logic MUST have unit tests (HTTP client adapters MAY use integration tests)
- Public functions MUST have doc comments explaining purpose and behavior
- Tauri IPC commands MUST be documented in code with input/output examples
- Clear module separation: `db.rs`, `rag.rs`, `local_embedding.rs`, `bookmark.rs`, `bookmark_exclusion.rs`
- No orphaned dependencies (run `cargo +nightly udeps` periodically)
- Breaking changes to Tauri commands MUST increment major version

**Rationale:** Code is read 10x more than written. Clippy catches bugs before users do.
Doc comments are living documentation that stays synchronized with code. Module boundaries
prevent the codebase from becoming a tangled mess as features grow.

### VI. Python Development Standards

**Non-Negotiable Rules:**

- Python 3.13+ MUST be used for all Python code
- All Python package management MUST use `uv` (not pip or poetry)
- Virtual environments (venv) MUST be used for dependency isolation
- TypedDict MUST be used for structured data (avoid plain `dict` where possible)
- All function arguments and return values MUST have explicit type hints
- The `Any` type MUST be avoided (use specific types or `Union`)
- All Python code MUST pass `mypy --strict` or `pyright` with zero errors
- All Python code MUST be formatted with `ruff format` before commit
- All Python code MUST pass `ruff check` with zero warnings

**Rationale:** Python's dynamic typing can hide bugs that only surface at runtime.
Strict type checking with mypy/pyright catches issues during development. TypedDict
provides structured data validation without class overhead. `uv` is significantly
faster than pip and provides better dependency resolution. Consistent formatting
with ruff reduces cognitive load during code review.

## Technical Constraints

### Simplicity Mandate

**Non-Negotiable Rules:**

- Use simple String types for data modeling unless strong typing prevents bugs
- Avoid premature abstraction (no trait until 3+ concrete implementations exist)
- No new features UNLESS explicitly requested by user or required for existing features
- Dependencies MUST be justified: explain what problem they solve and why custom code won't work
- Configuration MUST have sensible defaults (zero-config first run for basic use)

**Rationale:** Complexity is the enemy of maintainability. String typing reduces cognitive
load. YAGNI (You Aren't Gonna Need It) prevents feature creep. Dependencies are liabilities
(supply chain risk, compilation time, maintenance burden).

### Technology Stack

**Fixed:**

- Backend: Rust 1.75+ with Tauri 1.5+
- Frontend: Svelte 5+ with Vite
- Database: SQLite via rusqlite (bundled)
- Embeddings: Python 3.11+ with FastAPI, llama-cpp-python, and embeddinggemma-300m-qat GGUF model
- Package Management: `uv` for Python dependencies

**Variable (user-configurable):**

- Python embedding server port (default: 8000, via `EMBEDDING_SERVER_PORT` env var)

### Bundle Identity

- **Bundle Identifier**: `com.localmind.app` (MUST NOT change—breaks update mechanism)
- **Application Name**: `LocalMind` (user-visible, can localize in future)

## Governance

### Amendment Procedure

1. Proposed changes MUST be documented in issue or pull request
2. Rationale MUST explain why change is necessary (not just desirable)
3. Impact on existing code and user experience MUST be assessed
4. For MAJOR changes: breaking changes, principle removal/redefinition
5. For MINOR changes: new principle, expanded guidance, new mandatory section
6. For PATCH changes: clarifications, typo fixes, non-semantic improvements
7. Constitution version MUST be updated following semantic versioning
8. Templates MUST be updated to reflect new principles before merge

### Quality Gates

**Pre-Commit (Rust):**

- `cargo fmt --check` MUST pass
- `cargo clippy` MUST pass with zero warnings

**Pre-Commit (Python):**

- `ruff format --check` MUST pass (or run `ruff format` to auto-fix)
- `ruff check` MUST pass with zero warnings
- `mypy --strict` MUST pass with zero errors

**Pre-Release:**

- `cargo test` MUST pass all tests
- Manual smoke test on target platforms (Windows, macOS, or Linux)
- Tauri bundle MUST build successfully
- Python embedding server MUST start within 30 seconds and respond to health checks
- Version in `tauri.conf.json` MUST be updated per semantic versioning

### Compliance Review

- All pull requests MUST reference which principles they uphold
- Features that conflict with principles MUST be rejected or redesigned
- Dependency additions MUST justify necessity in PR description
- Breaking changes to Tauri IPC MUST be documented in CHANGELOG with migration guide

### Runtime Development Guidance

- See `localmind-rs/CLAUDE.md` for AI assistant guidance
- See `localmind-rs/README.md` for developer quickstart
- See `.specify/templates/` for feature planning templates

**Version**: 1.1.0 | **Ratified**: 2025-12-09 | **Last Amended**: 2025-01-21
