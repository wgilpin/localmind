# SvelteKit Migration Plan

### High-Level Plan

The migration will be an incremental process to minimize disruption. We will build the new SvelteKit frontend in a separate directory, and once it's feature-complete, we'll switch the backend to serve the new application.

### Detailed Steps

1. **Project Setup**:
    * A new SvelteKit project will be created inside the `desktop-daemon` directory, named `frontend`.
    * This keeps the new SvelteKit code isolated from the current `public` directory.

2. **Componentization**:
    * The existing UI will be broken down into the following Svelte components:
        * `Search.svelte`: Manages the search input and button.
        * `Results.svelte`: Displays the search results.
        * `NewNote.svelte`: Contains the form for adding a new note.
        * `FAB.svelte`: The floating action button for creating a new note.
        * A root `+layout.svelte` file will manage the overall page structure.

3. **Styling**:
    * The styles from `styles.css` will be migrated. Global styles (like CSS variables and body styles) will be placed in a global stylesheet, and component-specific styles will be scoped within each `.svelte` file.

4. **API Interaction**:
    * The API calls currently in `main.ts` for searching and saving notes will be recreated using SvelteKit's `fetch` API. We will leverage SvelteKit's form actions to handle the "save note" functionality and server-side `load` functions for fetching data where appropriate.

5. **Backend Integration**:
    * The SvelteKit application will be configured with `adapter-static` to output a static site.
    * The main Express server file, `desktop-daemon/src/index.ts`, will be modified to serve the static files from the new SvelteKit build output directory (`desktop-daemon/frontend/build`) instead of the current `desktop-daemon/public` directory. The existing API endpoints (`/search`, `/documents`) will remain unchanged.

### New Architecture

```mermaid
graph TD
    subgraph Browser
        A[SvelteKit Frontend]
    end

    subgraph "Server (Express.js in index.ts)"
        B[API Endpoint: /search]
        C[API Endpoint: /documents]
        D[Static File Server for SvelteKit build]
    end

    A -- HTTP Request --> B;
    A -- HTTP Request --> C;
    D -- Serves HTML/JS/CSS --> A;

    subgraph "Data Services"
        E[RAG Service]
        F[Document Store]
        G[Vector Store]
    end

    B --> E;
    C --> F;
    E --> G;
    E --> F;
