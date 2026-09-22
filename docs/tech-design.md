# Technical design — Agent Knowledge

## What this document is

The system's **structure**: which components exist, what each one is responsible for, what shape the data has, and how failures are categorized.

This document does not repeat what already lives elsewhere:

- **why** each choice was made, and what is still undecided → [technical decisions](./technical-decisions.md);
- **what behavior** is expected, as testable scenarios, and in what order to build it → [the behavior specification](../spec/SPEC.md);
- product scope, users and non-goals → [product requirements](./prd.md);

If anything here contradicts one of those, they take precedence — this file describes shape, not decision or behavior contract.

**History (2026-09-21):** the first version of this document designed a central instance reached over authenticated HTTPS. The same day, on clarifying that the author operates macOS and Linux sequentially — one machine at a time — that architecture was replaced by Git synchronization, with no server. See D-003, D-014 (superseded) and D-015.

## Technical goal

Deliver a Rust system that offers MCP-compatible agents and a human CLI the same per-project operational memory, continuous between macOS and Linux, without depending on any remote service beyond Git's standard transport.

## Architecture view

    Mac                                             memory repository (Git)
    Codex ─────── stdio ──▶ Rust core ──┐
    Claude Code ─ stdio ──▶ Rust core ──┼──▶ local clone ──┐
    CLI ──────────────────────────────────┘        │         │
                                                     │         │  pull / commit / push
                                  SQLite + FTS5 (local index) │
                                                               ▼
                                                        remote (e.g. GitHub)
                                                               ▲
                                  SQLite + FTS5 (local index) │
                                                     │         │  pull / commit / push
    Linux                                            │         │
    Codex ─────── stdio ──▶ Rust core ──┐          │         │
    Claude Code ─ stdio ──▶ Rust core ──┼──▶ local clone ──┘
    CLI ──────────────────────────────────┘

There is no network adapter or API of the product's own. Each machine runs the full core — identity, persistence, search, briefing, handoff — against its own copy of the memory repository and its own index. The only thing that crosses the network is the Git history, over the transport Git itself implements.

Practical consequence for testing: the core is entirely exercisable offline. An integration test runs two cores against two local clones of the same memory repository and checks behavior after a push/pull between them, with no server to simulate.

## Internal responsibilities

### Project identity

- locate the manifest from the working directory, in the code repository;
- validate its format and version;
- obtain the immutable project identifier;
- resolve, from that identifier, the corresponding directory inside the local memory repository;
- refuse operations when the project is not initialized or the manifest is invalid.

Path, directory name and the code repository's remote URL play no part in identity.

### Application core

Runs entirely local to each machine:

- create and update knowledge;
- mark knowledge obsolete and record supersession relations;
- search within the bound project, against the local index;
- assemble the briefing;
- logically replace the active handoff;
- coordinate synchronization and index rebuilding;
- convert internal failures into stable errors for the adapters.

The core does not know about JSON-RPC, terminal arguments or stdout. It also does not know which of the two machines it is running on — only that it operates over a local clone of the memory repository.

### Persistence

- read and write the memory repository's Markdown files, with structured frontmatter;
- open and configure the local SQLite index;
- rebuild that index entirely from the files, on demand or when detected stale/corrupted;
- run prepared statements for search;
- keep file and index consistent: a write is only complete locally once both have been updated and the commit created;
- implement expected revision, comparing the read state against the file's state after a pull.

The SQLite library used for the local index needs FTS5 enabled, on macOS and on Linux. Native access sits behind a safe Rust abstraction.

### Search

- treat ordinary input as text, without requiring FTS5 syntax;
- limit the query to the current project and to the local index of the machine in use;
- search title and content;
- order by textual relevance, with a limited priority adjustment and deterministic tie-breaking;
- include obsolete records only on request;
- signal when the local index may be stale, without blocking the search for it.

### Briefing

Selects, from the local index: pinned content, active high-priority decisions, active high-priority gotchas, and the active handoff. Applies a total limit, per-section quotas, stable ordering and deterministic tie-breaking.

### Handoff

Persists, as a Markdown file in the memory repository: completed work, pending work, recent decisions or limitations, and the concrete next step. Each project has exactly one active handoff; a new write deactivates the previous one in the same commit.

### Git synchronization

- clone the memory repository on the machine's first run;
- run `pull` before any read that feeds search or the briefing, and before any write;
- detect, on `pull`, whether the target file changed since it was last known-read — that is how "expected revision" works between machines;
- create atomic, deterministic commits, one per write, with a standardized message;
- try `push` after each commit and clearly report a connectivity failure, leaving the local commit intact;
- never force-push destructively, never rewrite history;
- use the Git credentials already configured on the machine, without managing secrets of its own.

This component replaces what, in the earlier version of this design, was the "central network adapter".

### MCP adapter

- carry out the initialization cycle and announce supported tools;
- validate input format and size;
- invoke the local core;
- convert results and failures into MCP responses;
- reserve stdout exclusively for the protocol, sending diagnostics only to stderr.

Processes one request at a time. There is no cross-machine coordination in the protocol itself — that happens through the Git repository.

### CLI adapter

Human operations over the same core: initialize and diagnose identity; create, query, search, update and obsolete knowledge; query the briefing and handoff; force synchronization; rebuild the index. Produces distinct exit codes per error category.

## Data topology

Each machine keeps a full clone of the memory repository and its own SQLite + FTS5 index, rebuilt from that clone. There is no central database: what is shared is the repository's history, through the remote the author configures.

Inside the memory repository, data is separated by directory, named after each project's immutable identifier. A record is a file; the active handoff is a file with a predictable marker, findable without scanning history.

The code repository carries only the identity manifest. No memory data lives inside it.

Without connectivity, reads use the last known local state and writes keep working locally. There is no queue of the product's own — the local commit already is the queue, visible through `git log` and `git status`.

## Conceptual model

### Project

- immutable identifier;
- identity format version;
- creation and update dates.

### Knowledge

Markdown file; frontmatter with:

- immutable identifier;
- title;
- category: decision, gotcha, procedure or note;
- state: active or obsolete;
- priority;
- pinned-content flag;
- monotonic revision;
- optional relation to the record it supersedes;
- creation and update dates.

The owning project is implicit from the directory. The file body is the record's Markdown content, preserved as written.

### Handoff

Markdown file; frontmatter with immutable identifier, state (active/inactive), monotonic revision and dates. The body holds the four content fields defined in scope.

### Text index (local, per machine)

Reference to the source file, owning project for mandatory filtering, indexed title and indexed content. Entirely derived; any repair rebuilds it from scratch.

## Error model

The contract between the core and the adapters. The core distinguishes at least:

- invalid input;
- project not initialized;
- invalid identity;
- record not found;
- revision conflict (file changed since it was last read);
- local index temporarily busy;
- synchronization unavailable (no connectivity or unreachable remote) — distinct from a conflict: the local write stays valid;
- local write pending synchronization;
- incompatible frontmatter schema;
- local index integrity failure (resolved by rebuilding, never by partial repair);
- local persistence failure (disk full, permission);
- MCP protocol failure;
- internal failure.

Operational errors must be actionable and must not include secrets or full record content. MCP and CLI convert these categories to their own contracts without exposing internal library details.
