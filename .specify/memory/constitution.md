<!--
Sync Impact Report
==================
Version change: 1.3.0 → 1.3.1
Bump rationale: PATCH — clarified Principle IV exclusion rule scope to bookmark tree
  folders only (not filesystem directories), preventing ambiguity with folder-watch
  ingestion features.
Modified principles:
  IV (Intelligent Automation with User Control) — exclusion rule scoped to bookmark
    tree folders and domains; filesystem directory watching is governed separately
Added sections: None
Removed sections: None
Templates requiring updates:
  ✅ .specify/templates/plan-template.md — no change required
  ✅ .specify/templates/spec-template.md — no change required
  ✅ .specify/templates/tasks-template.md — no change required
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
- Network requests ONLY to localhost:8000 (Python embedding server) — no external or
  remote requests
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
- Native window decorations and OS integration via eframe

**Rationale:** Desktop applications compete with instant web search. Sub-100ms responses
maintain user flow. Rust's memory safety prevents crashes that erode trust. Single
executable reduces friction for non-technical users.

### III. Modern UI/UX Excellence

**Non-Negotiable Rules:**

- UI MUST be accessible per WCAG 2.2 AA standards
- All user-facing text MUST be clear and jargon-free
- Error messages MUST be actionable (tell user what to do, not just what failed)
- Loading states MUST be shown for operations >200ms
- Keyboard navigation MUST be fully supported
- Dark theme MUST be applied by default (egui dark mode)
- UI state MUST be managed through egui's immediate mode paradigm

**Rationale:** Users interact with the UI hundreds of times daily. Accessible design
benefits all users, not just those with disabilities. egui's immediate mode provides
predictable UI updates without complex state management.

### IV. Intelligent Automation with User Control

**Non-Negotiable Rules:**

- Bookmark monitoring MUST be opt-in with clear explanation
- Users MUST be able to exclude bookmark tree folders and domains before bookmark
  ingestion starts (this rule applies to the bookmark tree, not filesystem directories)
- All automated actions MUST be visible in UI (e.g., "Monitoring 127 bookmarks")
- Search MUST degrade gracefully if embedding server unavailable (show cached results
  or clear error)
- No background data collection without active user session
- Users MUST be able to export all data in standard formats (JSON, CSV)
- Configuration MUST be editable via UI (no manual file editing required)

**Rationale:** Automation saves time, but invisible automation erodes trust. Users must
remain in control of their data. Graceful degradation ensures core search works even if
embedding server setup fails.

### V. Developer Quality & Maintainability

**Non-Negotiable Rules:**

#### Type Safety

- Use strong typing throughout — no untyped `HashMap<String, serde_json::Value>` as a
  substitute for real types
- Define explicit structs for all function arguments and return types where the shape
  is non-trivial
- Avoid `unwrap()` and `expect()` in library/service code; use `?` and explicit error
  types
- Use `thiserror` for defining error types; use `anyhow` only in binaries/main.rs
- Never use `Box<dyn Any>` or type-erase without a documented, compelling reason

#### Code Style

- Prefer functional style: iterators, `map`/`filter`/`fold` over manual loops where clear
- Avoid unnecessary OOP — use plain structs and free functions unless trait polymorphism
  is genuinely needed
- Keep modules small and focused; split files before they become unwieldy
- No dead code — `#[allow(dead_code)]` in committed code is a red flag

#### Architecture

- Separate concerns strictly: backend services (data, storage, business logic) MUST be
  distinct from egui frontend code
- Backend services MUST be pure and independently testable (no egui dependencies)
- egui components are NOT unit tested — UI is inherently hard to test headlessly
- Use channels (`std` or `tokio`) for all communication between async backend tasks and
  egui's synchronous update loop
- The egui update loop MUST NOT be blocked by I/O or long-running computation

#### Testing

- Apply TDD for all backend services and business logic
- No unit tests for egui components or rendering logic
- Never call remote APIs in tests — mock all external services
- Tests that rely on a live LLM or remote API are not valid; use recorded fixtures or
  mocks
- Use `mockall` or manual trait-based mocks; prefer trait objects to enable substitution
- Test database MUST never be the live database
- Automated backups MUST be scheduled on any live database

#### Tooling

- All code MUST pass `cargo clippy -- -D warnings` (zero warnings) before committing
- All code MUST be formatted with `cargo fmt` before saving
- Use `cargo check` frequently; do not accumulate type errors
- Run `cargo +nightly udeps` periodically to remove orphaned dependencies
- Public functions MUST have doc comments explaining purpose and behavior
- Breaking changes to public APIs MUST increment major version

**Rationale:** Code is read 10x more than written. Clippy and strict types catch bugs
before users do. Module boundaries and architectural separation prevent the codebase
becoming a tangled mess. TDD for services ensures correctness without brittle UI test
overhead.

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
Strict type checking catches issues during development. `uv` is significantly faster
than pip and provides better dependency resolution.

### VII. Observability & Logging

**Non-Negotiable Rules:**

- Use the `tracing` crate (not `log`) with `tracing-subscriber` for all logging
- Every error MUST be logged before being swallowed or converted
- No silent exceptions — if an error is handled it MUST be logged at `warn!` or
  `error!` level
- Use structured fields rather than format strings where possible:
  `tracing::info!(user_id = %id, "action")` not `tracing::info!("action for {id}")`
- Span context MUST be propagated across async task boundaries

**Rationale:** Observability is the only way to diagnose problems in a running desktop
app without a debugger attached. Structured fields make logs machine-queryable.
`tracing` integrates natively with tokio's task model.

### VIII. LLM / AI Integration

**Non-Negotiable Rules:**

- For Gemini models, ALWAYS use the 3-series (e.g. `gemini-3-flash-preview`); NEVER
  use the 2-series
- LLM APIs MUST NOT be called in tests — mock the trait or interface
- All LLM integrations MUST be behind a trait to enable substitution in tests
- Recorded fixtures or mocks MUST be used where test scenarios require LLM responses

**Rationale:** The 2-series Gemini models are superseded and may be deprecated. Calling
live LLMs in tests creates flaky, slow, and cost-incurring test suites. Trait-based
abstraction allows the test suite to run entirely offline.

## Technical Constraints

### Simplicity Mandate

**Non-Negotiable Rules:**

- This is a demo/prototype, not a production system — keep code as simple as possible
- Avoid premature abstraction (no trait until 3+ concrete implementations exist)
- No new features UNLESS explicitly requested by the user or required for existing
  features
- Dependencies MUST be justified: explain what problem they solve and why custom code
  won't suffice
- Configuration MUST have sensible defaults (zero-config first run for basic use)

**Rationale:** Complexity is the enemy of maintainability. YAGNI prevents feature creep.
Dependencies are liabilities (supply chain risk, compilation time, maintenance burden).

### Technology Stack

**Fixed:**

- Backend: Rust stable toolchain (via rustup) with Tokio async runtime
- Frontend: egui/eframe (pure Rust, no JavaScript)
- Database: SQLite via rusqlite (bundled)
- Embeddings: Python 3.13+ with FastAPI and google/embeddinggemma-300M model
- Package Management: `uv` for Python dependencies

**Variable (user-configurable):**

- Python embedding server port (default: 8000, via `EMBEDDING_SERVER_PORT` env var)

### Bundle Identity

- **Bundle Identifier**: `com.localmind.app` (MUST NOT change — breaks update mechanism)
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

### Explicit Approval Gates

The following actions MUST NOT proceed without explicit user approval:

- Adding any new dependency to `Cargo.toml`
- Adding any new feature or capability not described in the current spec
- Applying any schema migration on a live database

### Quality Gates

**Pre-Commit (Rust):**

- `cargo fmt --check` MUST pass (or run `cargo fmt` to auto-fix)
- `cargo clippy -- -D warnings` MUST pass with zero warnings

**Pre-Commit (Python):**

- `ruff format --check` MUST pass (or run `ruff format` to auto-fix)
- `ruff check` MUST pass with zero warnings
- `mypy --strict` MUST pass with zero errors

**Pre-Release:**

- `cargo test --all` MUST pass all tests
- Manual smoke test on target platforms (Windows, macOS, or Linux)
- Release binary MUST build successfully
- Python embedding server MUST start within 30 seconds and respond to health checks
- Version in `Cargo.toml` MUST be updated per semantic versioning

### Compliance Review

- All pull requests MUST reference which principles they uphold
- Features that conflict with principles MUST be rejected or redesigned
- Dependency additions MUST justify necessity in PR description
- Breaking changes to public APIs MUST be documented in CHANGELOG with migration guide

### Runtime Development Guidance

- See `localmind-rs/CLAUDE.md` for AI assistant guidance
- See `localmind-rs/README.md` for developer quickstart
- See `.specify/templates/` for feature planning templates

**Version**: 1.3.1 | **Ratified**: 2025-12-09 | **Last Amended**: 2026-04-07
