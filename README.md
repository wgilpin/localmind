# LocalMind

LocalMind is a project that allows you to store and search your notes locally. It consists of a desktop daemon that manages the data and a Chrome extension for interacting with it.

## Overview

The project is structured as a monorepo with two main components:

- `desktop-daemon`: A Node.js server that handles data storage, indexing, and search. It uses Express for the API, and a local file-based database.
- `chrome-extension`: A Chrome extension that provides a user interface for creating and searching notes.

## Installation

### Prerequisites

- Node.js (v18 or higher)
- npm

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

### Start the server

To start the server, run the following command from the `desktop-daemon` directory:

```bash
npm run dev
```

### Start the frontend

To start the frontend, run the following command from the `desktop-daemon/frontend` directory:

```bash
npm run dev
