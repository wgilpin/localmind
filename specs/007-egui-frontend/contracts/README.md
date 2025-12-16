# Contracts: Native egui Desktop GUI

**Feature**: 007-egui-frontend

## No New External APIs

This feature does not introduce new external APIs. The existing HTTP API on port 3000-3010 remains unchanged for Chrome extension compatibility.

## Internal Changes

- Tauri IPC commands are replaced with direct Rust function calls
- No new contracts required
- HTTP server endpoint signatures unchanged

## Existing API Reference

See `specs/001-http-api-server/contracts/openapi.yaml` for the HTTP API specification.


