# Product Requirements Document – LocalMind

**Document owner:**&#x20;

**Last updated:** 2025‑07‑07

---

## 1. Purpose & Vision

People save countless webpages and notes but struggle to recall them quickly. The LocalMind combines bookmark capture, note‑taking, and Retrieval‑Augmented Generation (RAG) search into a private, device‑local knowledge base.

> **Vision:** “Whatever you’ve read or written is instantly retrievable and answerable, without your data ever leaving your machine.”

---

## 2. Problem Statement

- Browser bookmarks capture only URLs; context is lost.
- Note apps store thoughts but are siloed from browsing history.
- Cloud search tools raise privacy concerns; offline access is unreliable.
- On‑device LLMs have matured enough to run practical RAG workflows locally.

---

## 3. Goals & Non‑Goals

| Goals                                                                                                | Non‑Goals                                                                         |
| ---------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| Capture full page content and metadata with one click.                                               | Cross‑user collaboration or shared vaults (future).                               |
| Allow free‑form notes similar to Google Keep.                                                        | Mobile‑only local LLM inference (initial version).                                |
| Maintain a single encrypted local vector & document store.                                           | Large‑scale web clipping (>100k docs) optimisation.                               |
| Natural‑language & semantic search powered by local RAG.                                             | Polishing advanced summarisation, clustering, etc.                                |
| Optional phone‑to‑desktop ingestion via relay server; data stored only after reaching master device. | Real‑time multi‑device conflict resolution (initially rely on “last write wins”). |

---

## 4. User Personas

1. **Research Power User (primary)** – Hunts across dozens of academic / technical sources daily and needs instant recall.
2. **Curious Casual (secondary)** – Saves recipes and articles, wants privacy and simplicity.
3. **Mobile Nomad** – Discovers links on phone, expects them to appear on deskntop without manual copy/paste.

---

## 5. User Stories (selected)

- *As a* researcher *I want* to clip an article and later ask, “What did that paper conclude about attention spans?” *so that* I can cite it quickly.
- *As a* user reading on phone *I want* to share a link to the app *so that* it is indexed on my laptop automatically.
- *As a* privacy‑conscious user *I want* local processing only *so that* no third party sees my data.
- *As a* content creator (e.g. LinkedIn or internal blogs) *I want* to find related saves for my topic suggestions *so that* I can craft posts on the fly.

---

## 6. Functional Requirements

### 6.1 Chrome Extension

- One‑click save → captures HTML, plain text, canonical URL, timestamp, and favicon.
- Auto‑extract title, description, OpenGraph tags.
- Suggest tags automatically from page content (e.g., [AI], [Recipes]) and allow user acceptance or editing.

### 6.2 Notes

- Quick note composer (title + body + optional tags).
- Stored identically to page clips; supports markdown.

### 6.3 Local RAG Store

- Documents chunked & embedded using the fixed 384‑dim sentence‑transformer **all‑MiniLM‑L6‑v2** (served via Ollama). The embedding model is not user‑configurable to avoid expensive re‑embedding of the entire corpus.
- Vector index: FAISS flat (MVP) with metadata filters.
- Content and index encrypted at rest (AES‑256, key derived from OS keychain).

### 6.4 Search UI

- Web app (Electron / local host) with a single input.
- Pipeline: user query → **all‑MiniLM‑L6‑v2** embeddings → similarity search (top‑k) → context prompt → local LLM served by **Ollama** (auto‑selects the best model the device can run, e.g., Llama 3 8B‑Q4) → answer + citations list.
- Toggle between “chat” (RAG) and “keyword” modes.

### 6.5 Cross‑Device Ingestion

- Lightweight relay server (golang) receives shares from the installed **Mobile Share App** (iOS Share Extension / Android Share Target); stores encrypted blob ≤24 h.
- Desktop client polls and downloads, then erases blob.

---

## 7. Non‑Functional Requirements

- **Privacy:** Data never leaves user devices unencrypted; relay server stores only blobs & device IDs.
- **Performance:** <300 ms median retrieval for vault ≤10k docs on M1 MacBook Air.
- **Offline‑first:** Full functionality without internet after initial setup.
- **Security:** OWASP ASVS L2 compliance; signed binaries.
- **Accessibility:** WCAG 2.2 AA for web UI.

---

## 8. Technical Architecture

```
[Chrome Extension / Mobile Share App]──(HTTPS)──►[Relay Server]──►[Desktop Daemon]
                                               │
             [Web UI]◄──local HTTP──[Desktop Daemon]──►[Vector DB (FAISS)]
                                               │
                                        [Ollama Runtime]
```

- Daemon exposes gRPC for extension and UI.
- Embeddings cached, re‑computed on update.
- Automatic incremental backups (optional) to user‑chosen location.
- **Ollama runtime optimisation:** To prevent model swap latency when alternating between the tiny embedding model and a larger chat model:
  - Set environment variables `OLLAMA_KEEP_ALIVE=-1` (never unload) and `OLLAMA_MAX_LOADED_MODELS=2`.
  - If memory is tight, run two Ollama instances on separate ports—e.g., 11434 for `all‑MiniLM‑L6‑v2` (CPU‑only) and 11435 for the chat model (GPU)—so both stay resident without contention.
  
Local web app served from:


---

## 9. MVP Scope (v0.1)

- Chrome desktop extension.
- Desktop daemon with FAISS + Ollama.
- Basic web UI with chat search.
- Manual phone‑to‑desktop via share‑to‑relay.
- No tagging or encryption UI (hard‑coded key).

---

## 10. Success Metrics

| Metric                             | Target by v1.0 |
| ---------------------------------- | -------------- |
| Median end‑to‑end answer latency   | ≤1 s           |
| Recall\@5 for “known item” queries | ≥95 %          |
| Weekly Active Clippers (WAC)       | 500 beta users |
| Crash‑free sessions                | 99.5 %         |

---

## 11. Assumptions

- Users comfortable installing 1‑2 GB local models.
- Device has ≥8 GB RAM.

---

## 12. Open Questions

1. Which license‑compatible local models provide best quality vs. size?
2. How to handle large PDFs & epubs?
3. Do we surface passive recommendations (“You saved X last year”)?

---

## 13. Risks & Mitigations

| Risk                            | Likelihood | Impact | Mitigation                        |
| ------------------------------- | ---------- | ------ | --------------------------------- |
| Model quality insufficient      | Medium     | High   | Allow user to swap models         |
| Extension store approval delays | Medium     | Medium | Submit early, long review buffer  |
| Relay server breach             | Low        | High   | Zero‑knowledge blobs; auto‑delete |

---

## 14. Future Enhancements

- iOS/Android native clipper with on‑device indexing.
- Summarisation of collections.
- Browser sidebar with contextual suggestions.

---

