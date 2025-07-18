# LocalMind

LocalMind is a project that allows you to store and intelligently earch your notes and bookmarks locally. It consists of a desktop daemon that manages the data and a Chrome extension for interacting with it. It leverages a Retrieval-Augmented Generation (RAG) architecture with Ollama to provide intelligent search and summarization capabilities.

## Overview

The project is structured as a monorepo with two main components:

- `desktop-daemon`: A Node.js server that handles data storage, indexing, and search. It uses Express for the API, and a local file-based database.
- `chrome-extension`: A Chrome extension that provides a user interface for creating and searching notes.

## Installation

### Prerequisites

- Node.js (v18 or higher)
- npm
- Ollama

### Ollama Installation

Ollama is required to run the large language models locally.

**Windows:**

1.  Download the installer from the [Ollama website](https://ollama.com/download).
2.  Run the downloaded `.exe` file and follow the installation instructions.

**macOS:**

1.  Download the Ollama app from the [Ollama website](https://ollama.com/download).
2.  Unzip the downloaded file and drag `Ollama.app` to your Applications folder.

**Linux:**

Run the following command in your terminal:

```bash
curl -fsSL https://ollama.com/install.sh | sh
```

### Desktop Daemon

1. Navigate to the `desktop-daemon` directory:

    ```bash
    cd desktop-daemon
    ```

2. Install the dependencies:

    ```bash
    npm install
    ```

### Frontend

1. Navigate to the `desktop-daemon/frontend` directory:

    ```bash
    cd desktop-daemon/frontend
    ```

2. Install the dependencies:

    ```bash
    npm install
    ```

### Chrome Extension

1. Open Chrome and navigate to `chrome://extensions`.
2. Enable "Developer mode".
3. Click "Load unpacked" and select the `chrome-extension` directory.

## Running the application

### Start the Application

To start both the Ollama environment and the LocalMind server, run the following command from the project root directory:

```bash
./start_dev.sh
```

This script will:
1. Ensure Ollama is running and set `OLLAMA_NUM_PARALLEL=2` to enable concurrent model loading (preventing delays when switching between embedding and language models).
2. Install Node.js dependencies for the desktop daemon.
3. Start the LocalMind server in development mode with auto-reloading.

### Start the Frontend (if running separately)

If you need to start the frontend independently (e.g., for frontend-only development), run the following command from the `desktop-daemon/frontend` directory:

```bash
npm run dev
```