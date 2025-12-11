<!--
Sync Impact Report
==================
Version change: INITIAL → 1.0.0
Modified principles: N/A (initial constitution)
Added sections:
  - Core Principles (5 principles)
  - Technical Constraints
  - Governance
Removed sections: N/A
Templates requiring updates:
  ✅ .specify/templates/plan-template.md - reviewed, compatible
  ✅ .specify/templates/spec-template.md - reviewed, compatible
  ✅ .specify/templates/tasks-template.md - reviewed, compatible
Follow-up TODOs: None - all placeholders filled
-->

# LocalMind-rs Constitution

## Core Principles

### I. Privacy & Offline-First Architecture

**Non-Negotiable Rules:**

- All data processing MUST occur locally on the user's device
- SQLite database MUST use bundled rusqlite with no external dependencies
- Data MUST be encrypted at rest in `~/.localmind/` (Windows: `%APPDATA%/localmind/`)
- Zero cloud dependencies for core functionality
- Network requests ONLY to local LLM endpoints (Ollama/LM Studio)
- No telemetry, analytics, or external data transmission without explicit user consent
- Zero-knowledge architecture: relay servers (if implemented) MUST NOT have access to 
  unencrypted user data

**Rationale:** Users trust LocalMind with sensitive research and personal notes. Privacy is
not a feature but the foundation. Offline-first ensures reliability independent of network
conditions and protects against service disruptions.

### II. Performance & Native Experience

**Non-Negotiable Rules:**

- Search responses MUST target <100ms median latency
- Memory footprint MUST target <50MB for core application
- Single executable deployment (no installer sprawl)
- Rust async runtime (Tokio) for all I/O operations
- Streaming responses MUST be used for LLM outputs (no blocking)
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
- AI features MUST degrade gracefully if LLM unavailable (show raw search results)
- No background data collection without active user session
- Users MUST be able to export all data in standard formats (JSON, CSV)
- Configuration MUST be editable via UI (no manual file editing required)

**Rationale:** Automation saves time, but invisible automation erodes trust. Users must
remain in control of their data. Graceful degradation ensures core search works even if
LLM setup fails.

### V. Developer Quality & Maintainability

**Non-Negotiable Rules:**

- All code MUST pass `cargo clippy` with no warnings
- All code MUST be formatted with `cargo fmt` before commit
- All new modules MUST have unit tests for core logic
- Public functions MUST have doc comments explaining purpose and behavior
- Tauri IPC commands MUST be documented in code with input/output examples
- Clear module separation: `db.rs`, `rag.rs`, `ollama.rs`, `bookmark.rs`, `bookmark_exclusion.rs`
- No orphaned dependencies (run `cargo +nightly udeps` periodically)
- Breaking changes to Tauri commands MUST increment major version

**Rationale:** Code is read 10x more than written. Clippy catches bugs before users do.
Doc comments are living documentation that stays synchronized with code. Module boundaries
prevent the codebase from becoming a tangled mess as features grow.

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
- LLM: Ollama or LM Studio (user choice, both OpenAI-compatible API)

**Variable (user-configurable):**

- Embedding model (via LLM backend)
- Chat model (via LLM backend)
- LLM endpoint URL

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

**Pre-Commit:**

- `cargo fmt --check` MUST pass
- `cargo clippy` MUST pass with zero warnings

**Pre-Release:**

- `cargo test` MUST pass all tests
- Manual smoke test on target platforms (Windows, macOS, or Linux)
- Tauri bundle MUST build successfully
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

**Version**: 1.0.0 | **Ratified**: 2025-12-09 | **Last Amended**: 2025-12-09
