# Feature Specification: Graceful Model Startup

**Feature Branch**: `002-graceful-model-startup`  
**Created**: December 11, 2025  
**Status**: Draft  
**Input**: User description: "being able to change embeddings model from the UI means the UI needs to start even if it cant find the configured one. Currently I cant start the UI because nomic is not available"

## Clarifications

### Session 2025-12-11

- Q: When a user switches embeddings models, what should happen to existing bookmarks that were indexed with the previous model? → A: System marks existing bookmarks as "needs re-indexing" with a background queue that gradually re-embeds them automatically
- Q: Should the system support multiple embeddings model provider types (e.g., LM Studio, Ollama, OpenAI-compatible APIs) simultaneously, or focus on a single provider? → A: Support only one provider type but design architecture to be provider-agnostic for future expansion

## User Scenarios & Testing *(mandatory)*

### User Story 1 - UI Starts Without Configured Model (Priority: P1)

A user has configured an embeddings model (e.g., nomic) in their settings, but that model is no longer available (uninstalled, service down, renamed). When they launch the application, the UI starts successfully with a clear warning banner indicating the model issue. The user can immediately access settings to select a different available model, without needing to manually edit configuration files or reinstall the application.

**Why this priority**: This is blocking functionality. Users are currently unable to start the application if their configured model is unavailable, making the application unusable and requiring technical workarounds.

**Independent Test**: Can be fully tested by configuring a non-existent model name in settings, restarting the application, and verifying the UI starts with appropriate warnings and allows model selection.

**Acceptance Scenarios**:

1. **Given** a user has configured an embeddings model that is no longer available, **When** they launch the application, **Then** the UI starts successfully with a prominent warning banner about the unavailable model
2. **Given** the UI has started in degraded mode due to unavailable model, **When** the user navigates to settings, **Then** they can view all available models and select a new one
3. **Given** the user selects a new available model, **When** they save the settings, **Then** the application initializes the new model and removes the warning banner
4. **Given** the configured model becomes unavailable while the app is running, **When** an operation requires the model, **Then** a warning appears and guides the user to change models rather than crashing
5. **Given** no embeddings models are available on the system, **When** the user starts the application, **Then** the UI starts with a clear message explaining that at least one model must be installed, with links to installation instructions

---

### User Story 2 - Model Status Visibility (Priority: P2)

A user wants to understand which embeddings models are available and which one is currently configured. When they open the settings panel, they see a list of all detected models with clear status indicators (available, configured, unavailable). This helps them make informed decisions about which model to select based on availability and system resources.

**Why this priority**: Improves user experience and reduces confusion, but the core functionality works without detailed status information.

**Independent Test**: Can be tested by having multiple models installed, configuring one, and verifying the UI shows accurate status for each model.

**Acceptance Scenarios**:

1. **Given** the user opens model settings, **When** the settings panel loads, **Then** all detected embeddings models are listed with clear availability status
2. **Given** multiple models are available, **When** viewing the model list, **Then** the currently configured model is clearly marked
3. **Given** a model is available but not installed/ready, **When** viewing the list, **Then** the model shows an appropriate status with action options (install, download, etc.)
4. **Given** model availability changes while settings are open, **When** the system detects the change, **Then** the status updates without requiring page refresh

---

### User Story 3 - Graceful Feature Degradation (Priority: P2)

A user launches the application without a working embeddings model to quickly check their existing bookmarks and history. While embedding-dependent features (semantic search, similarity) are disabled, they can still browse their bookmark collection, use text-based search, and access settings. This allows partial productivity while resolving the model issue.

**Why this priority**: Enhances resilience but is not critical for fixing the immediate blocking issue of changing models.

**Independent Test**: Can be tested by starting with no configured model and verifying that non-embedding features remain functional.

**Acceptance Scenarios**:

1. **Given** the application starts without a working model, **When** the user attempts to use semantic search, **Then** a clear message explains the feature requires a model and provides a quick link to model settings
2. **Given** no embeddings model is available, **When** the user browses bookmarks or history, **Then** these features work normally without embedding-dependent functionality
3. **Given** embedding features are disabled, **When** the UI displays feature buttons, **Then** disabled features are visually marked with tooltips explaining why they're unavailable
4. **Given** the user switches to an available model, **When** the model initializes successfully, **Then** all embedding-dependent features automatically become available without restart

---

### Edge Cases

- When the configured model exists but fails to initialize (corrupted, incompatible version), system treats it as unavailable and follows graceful degradation path
- When model availability check times out during startup, system assumes unavailable and allows UI to load rather than blocking indefinitely
- When multiple model names are configured (corrupted config), system validates each and uses the first available one or defaults to degraded mode
- When model configuration file is missing or corrupted, system starts in degraded mode with default empty config
- When user selects a model that becomes unavailable before initialization completes, system shows error and allows re-selection
- When system resources are insufficient for the selected model, clear error message indicates resource constraints and suggests lighter alternatives
- When the embeddings model provider service is not running, system detects this and provides guidance on starting the service
- When switching models, system ensures clean shutdown of previous model to prevent resource leaks
- When configuration contains a model name with typos or incorrect casing, system performs fuzzy matching to detect likely intended model
- When background re-indexing is in progress and user closes the application, system persists queue state and resumes on next startup
- When user switches models multiple times before re-indexing completes, system cancels the previous re-indexing queue and starts fresh with the newest model
- When re-indexing a bookmark fails (corrupted content, model unavailable), system logs error and continues with next bookmark in queue
- When system resources are constrained, background re-indexing automatically throttles to prevent impacting foreground operations

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST allow UI startup regardless of configured embeddings model availability
- **FR-002**: System MUST perform embeddings model availability check during startup without blocking UI initialization
- **FR-003**: System MUST display a persistent, dismissible warning banner when configured model is unavailable
- **FR-004**: System MUST provide a "degraded mode" state where non-embedding features remain functional
- **FR-005**: Model settings panel MUST be accessible even when no model is configured or available
- **FR-006**: System MUST detect and list all available embeddings models from the configured provider on startup
- **FR-007**: Model selection UI MUST show real-time availability status for each detected model
- **FR-008**: System MUST validate model availability before attempting initialization
- **FR-009**: System MUST handle model initialization failures gracefully without crashing the application
- **FR-010**: System MUST allow users to select and switch to any available model from the UI
- **FR-011**: When no models are available, system MUST display clear instructions on installing/configuring models
- **FR-012**: System MUST disable embedding-dependent features (semantic search, similarity) when no model is available
- **FR-013**: System MUST clearly indicate which UI features are unavailable due to missing model
- **FR-014**: System MUST automatically enable embedding-dependent features when a model becomes available
- **FR-015**: System MUST persist model selection changes immediately without requiring application restart
- **FR-016**: Warning messages MUST include actionable guidance (e.g., "Go to Settings → Models" button)
- **FR-017**: System MUST log detailed model initialization errors for troubleshooting while showing user-friendly messages in UI
- **FR-018**: Model availability check MUST timeout after reasonable period (5 seconds) to prevent startup delays
- **FR-019**: System MUST handle concurrent model operations (switching while another operation is in progress) safely
- **FR-020**: When user switches embeddings models, system MUST mark all existing bookmarks as "needs re-indexing"
- **FR-021**: System MUST provide a background queue that automatically re-embeds marked bookmarks with the new model
- **FR-022**: Background re-indexing queue MUST be pausable and resumable by the user
- **FR-023**: System MUST show re-indexing progress indicator (e.g., "Re-indexing: 45/200 bookmarks")
- **FR-024**: Bookmarks pending re-indexing MUST remain accessible for browsing and text search during the re-indexing process

### Key Entities

- **Model Status**: Represents the current state of an embeddings model with attributes: name, availability (available/unavailable/initializing/failed), configured (boolean), error message (if any), resource requirements
- **Application State**: Extended to include degraded mode flag indicating whether embedding features are available
- **Model Configuration**: Persisted settings including selected model name, initialization parameters, last known working model
- **Re-indexing Queue**: Tracks bookmarks requiring re-embedding after model switch with attributes: total count, completed count, paused status, current bookmark being processed, estimated time remaining

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Application UI starts within 3 seconds regardless of configured model availability
- **SC-002**: Users can successfully change embeddings model from unavailable to available within 30 seconds from UI startup
- **SC-003**: 100% of non-embedding features remain functional when model is unavailable
- **SC-004**: Warning messages clearly indicate model issues and next steps in less than 20 words
- **SC-005**: Model switching completes within 10 seconds for locally available models
- **SC-006**: Zero application crashes occur due to model unavailability or initialization failures
- **SC-007**: Model availability detection completes within 5 seconds on startup
- **SC-008**: Background re-indexing processes at least 10 bookmarks per minute without impacting UI responsiveness

## Assumptions *(optional)*

- Users have at least one embeddings model installed or can install one when prompted
- Model availability can be determined without full initialization (lightweight check available)
- Non-embedding features (bookmark browsing, text search) do not depend on embedding model
- Model switching can be performed without requiring full application restart
- Users understand that semantic search and similarity features require an embeddings model
- Embeddings model providers support availability checking without full model initialization
- Application has appropriate error logging system for debugging model issues
- Users can access external documentation or installation guides when needed
- System uses a single embeddings model provider type (architecture designed to be provider-agnostic for future multi-provider support)
- The configured provider service exposes a consistent API for model listing, availability checking, and initialization

## Dependencies *(optional)*

- Model detection system must have reliable method to query available models from configured services
- Application must support dynamic model initialization and switching without restart
- Settings UI must be modular enough to load independently from model initialization
- Application state management must support degraded/partial functionality modes
- Embeddings model providers must support availability status queries
- Configuration persistence layer must handle missing or corrupted config files gracefully

## Out of Scope *(optional)*

- Automatic downloading or installation of embeddings models
- Model performance benchmarking or recommendations
- Automatic model switching based on system resources or performance
- Support for multiple simultaneous embeddings models
- Support for multiple embeddings model provider types simultaneously (single provider in MVP)
- Provider selection UI (provider is configured at deployment/setup time)
- Model versioning and compatibility checking
- Cloud-based or remote embeddings model support
- Automatic repair or reinstallation of corrupted models
- Background model pre-loading or caching for faster switching
- Manual batch re-indexing UI with bookmark selection (automatic queue-based re-indexing is in scope)

