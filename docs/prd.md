# Product requirements — Agent Knowledge

## Summary

Agent Knowledge is operational memory for a code repository, shared between the author's macOS and Linux machines through a Git repository dedicated to memory data — no central server, no always-on instance. It lets AI agents recover a project's essential context and record durable knowledge across sessions and across machines: decisions, traps, procedures, and where the work stopped.

The goal is not to build a broad, general-purpose memory platform for agent teams, nor an agent observability platform. It is to solve, in a small and verifiable way, the loss of context when starting a new session, switching agents, switching machines, or resuming a task days later. The result must be a functional, reliable tool for personal and professional use — not just a programming exercise.

The first product is written in Rust. That allows learning applied systems programming while solving the real problem, without subordinating usefulness, data integrity, compatibility or maintainability to the exercise of manual memory management. The implementation is meant to be followed in detail: every module, ownership boundary, system resource, native dependency and architectural choice must be explainable.

How this scope was reached: [context and brainstorming](./context-and-brainstorming.md). Implementation choices: [technical decisions](./technical-decisions.md). Component structure: [technical design](./tech-design.md). The acceptance criteria below are expressed as testable scenarios in [the behavior specification](../spec/SPEC.md).

## Problem

An agent opening a repository does not automatically know:

- the relevant architecture;
- decisions already taken and why;
- errors, limits and traps already hit;
- procedures that worked;
- the state of the previous task and its next steps.

Re-explaining that context in every chat costs time, introduces inconsistency, and makes switching between Codex, Claude Code or other MCP-compatible agents unreliable. The problem is not solved if the knowledge only works for one agent, one session, or one machine.

## Product goal

For each stably identified repository, any configured MCP agent — on either machine — must be able to:

1. Get a short, deterministic briefing when starting a task.
2. Search existing knowledge before making a relevant decision.
3. Record knowledge that deserves to outlive the session.
4. Leave and recover a short handoff between sessions and between machines.

The system must stay useful without calls to external LLMs, vectors, automatic prompt capture, agent hooks, or any network infrastructure of its own. Synchronization between macOS and Linux reuses a mechanism the author already operates and trusts — Git — instead of introducing a new service to solve it; see technical decision D-015.

The main success criterion is real continuity of work across agents, sessions and machines. Learning systems fundamentals and Rust during implementation is a complementary goal, and it does not justify including mechanisms with no product value nor accepting a fragile solution.

## Initial user

A developer who works across several local repositories, alternates between code agents, and uses two machines — macOS and Linux — **one at a time**, never both in parallel on the same project in the same window of time. That premise of sequential, non-simultaneous use is what makes Git synchronization sufficient; see D-015. Agents are the primary clients over MCP, but the developer also needs to inspect, search, export and diagnose the data through a CLI.

## Main flow

```text
Task starts
  → agent gets the project briefing and the active handoff
  → agent searches memory before important decisions
  → agent records decisions, gotchas or reusable procedures
  → agent writes a handoff before finishing
```

That behavior is driven by project instructions given to the agent. In the first version there are no agent hooks capturing events automatically, and no automatic end-of-session reminders — the author works with several different agents and does not want a mechanism tied to one agent host.

## Functional scope of the first version

### Knowledge per project

Every record belongs to a project. Project identity is an immutable identifier, created explicitly and persisted in a small versioned manifest inside the code repository itself. The absolute path, the directory name and the remote URL may all change without creating another memory. Cloning or deriving a project with a different identity requires an explicit operation, to avoid sharing knowledge by accident.

The memory content itself — records and handoffs — does not live inside the code repository. It lives in a Git repository dedicated to memory data, with one directory per immutable project identifier. The code repository carries only the identity manifest, which points at that directory.

The MCP process starts bound to the project resolved from the working directory. Tools of normal use do not accept an arbitrary project on each call. That reduces the risk of accidental cross-repository reads and writes.

Records carry at least:

- title and content in Markdown;
- category: decision, gotcha, procedure or note;
- state: active or obsolete;
- creation and update dates;
- priority, for content that should appear in the briefing;
- an optional supersession relation, when new information invalidates older information.

### Briefing

The briefing does not use an LLM to decide what to show. It is assembled by explicit rules from a local index rebuilt from the synchronized records:

- pinned content;
- active high-priority decisions and gotchas;
- the project's active handoff.

It must have an explicit size limit, per-section quotas, stable ordering and deterministic tie-breaking. The same set of synchronized files and the same limits must produce the same briefing, sized to fit inside an agent's context.

### Search

Search is textual, through SQLite with FTS5 — an index kept locally on each machine and rebuilt from the synchronized Markdown files, never synchronized itself. It must search title and content only within the current project and return results ordered first by textual relevance, with limited influence from priority and deterministic tie-breaking. Ordinary search treats the input as text, without requiring the agent to know FTS5's internal syntax.

### Recording knowledge

The agent can create and update structured records. A locally successful write stores the Markdown file, updates the local index and creates a commit — atomically: either all three happen, or none does. Making that write visible to the other machine also requires a successful push; if the push fails for lack of connectivity, the local write stays valid and visible locally (never silently lost), but the system states explicitly that it is not yet synchronized.

Updates use an expected revision to detect concurrency: before writing, the client synchronizes with the remote repository and checks whether the record changed since it was last read. If it changed, the write is refused with an explicit conflict — it never silently accepts overwriting a version that was not read.

The system must not remove knowledge automatically. When something stops being true, it is marked obsolete or superseded explicitly.

### Handoff

There is one active handoff per project, containing:

- completed work;
- pending work;
- recent decisions or limitations;
- a concrete next step.

Writing a new handoff makes it the only active one. The previous one stays as an inactive file, recoverable, but browsing and analyzing session history is not part of the first version.

### Interfaces

The same core is reachable, on macOS and on Linux, through:

- a local MCP server over stdio, for compatible agents;
- CLI commands for diagnostics, search, export and manual synchronization.

In the first version, each client starts its own local MCP process and ends it with the session. So it is an MCP server, but not a persistent daemon — and there is no persistent process on either end. Durability and cross-machine continuity come from the Git repository, not from a running service. Different agent processes on the same machine can reach the same local index; the core applies limited, explicit concurrency control in that case.

In MCP mode, stdout is reserved exclusively for the protocol. Diagnostics go to stderr, without logging secrets or sensitive content by default.

## Out of scope for the first version

- Automatic capture through hooks of prompts, tools and sessions, and any automatic end-of-session reminder specific to one agent host.
- Automatic consolidation by LLM.
- Embeddings, vector search, entity graphs and hybrid reranking.
- Web interface, public HTTP API, users, teams, RBAC, OIDC, or any always-on central instance. Synchronization between machines is solved by Git's own transport (SSH or HTTPS to the remote), not by a protocol or service this product builds and hosts.
- Genuinely simultaneous use of macOS and Linux on the same project in the same window of time. The author confirms operating one machine at a time; that is the limit that makes Git synchronization sufficient instead of requiring real-time visibility.
- Automatic content merging when two machines edit the same record without synchronizing between them — that case becomes an explicit revision conflict, not an attempt to reconcile text.
- Git internal to the product (it uses an ordinary Git repository, it does not implement its own versioning) and file watchers outside the agent's explicit read/write flow.
- Automatic deletion, decay or automatic retention.
- Proprietary adapters for agents that do not support the adopted MCP transport.

## Non-functional requirements

- It must solve the real continuity flow across agents and between macOS and Linux; Rust is the chosen language to balance operational reliability, native distribution and systems learning.
- It must run entirely as a native executable on macOS and Linux, without depending on any host or service beyond the Git remote the author already uses — with reproducible installation and updates on each machine.
- It must store knowledge durably and readably as Markdown files versioned by Git; the memory repository's commit history is itself a form of durability and recovery.
- It must keep, on each machine, a local SQLite index with FTS5, rebuildable at any time from the synchronized files, never treated as the source of truth nor synchronized directly.
- Local writes work without connectivity (local commit); making a write visible to the other machine requires successful synchronization, and the system must clearly indicate when a write is stored locally but not yet synchronized — never lose it, never hide it.
- It must keep the data readable directly as Markdown files; the CLI offers query and diagnostics over those same files, without requiring a separate export for human inspection.
- It must be safe against large or malformed input, partial local writes and unsafe native boundaries.
- It must guarantee, on macOS and on Linux, the same supported SQLite version with FTS5 for each machine's local index, without assuming the capabilities of the system library.
- It must detect concurrent edits of the same record and respond with an actionable error, without silent loss of updates.
- The domain and persistence core must use safe Rust. Any use of unsafe Rust is restricted to an FFI or operating-system boundary, with a safety justification, encapsulation and a specific test.
- Dependencies, resource ownership, system resources and native libraries must have a justification the maintainer can understand. The Cargo lockfile is part of the reproducible distribution.

## Acceptance criteria for the first version

These are product-level criteria. Each one has one or more corresponding scenarios in [the behavior specification](../spec/SPEC.md) — that is where they become tests; here they state what the product promises.

1. Codex and Claude Code, in independent MCP configurations on the same machine, get the same briefing for the same project.
2. One agent creates a decision and another agent finds it by textual search in a later session on the same machine.
3. The same project stays associated with the same memory after changing path or being opened from another checkout that preserves its identity manifest.
4. Two projects neither read nor alter each other's records through the MCP tools of normal use.
5. A handoff created at the end of a session appears in the briefing of the next session for the same project, and the previous handoff stays recoverable.
6. An obsolete record stops appearing in the briefing but stays queryable.
7. The briefing respects its limit and stays byte-for-byte deterministic for the same set of synchronized records and the same configuration.
8. Two concurrent updates against the same revision result in one write and one explicit conflict, never a silent overwrite.
9. The local index can be deleted and rebuilt entirely from the synchronized Markdown files, with no loss of content or metadata.
10. Large or malformed MCP input does not bring down the process, does not contaminate stdout, and returns a structured error.
11. A decision recorded by an agent on macOS is found by an agent on Linux after a synchronization — and vice versa — using only Git's standard transport, with no server or central process belonging to the product.
12. A lack of connectivity at synchronization time produces an explicit, actionable error, but does not prevent the local write nor lose the record: it stays committed locally until the next successful synchronization.
13. The same functional and integration suite passes on macOS and Linux with FTS5 enabled.
14. Formatting, lint, unit tests, integration tests and dependency audit are part of validation from the first increment. Unsafe Rust and FFI boundaries have specific error tests; exposed parsers get property tests or fuzzing before the interface stabilizes.
15. A pilot in at least two real repositories proves that switching agents and switching machines recovers decisions and the stopping point without manually re-explaining what was already recorded.

## Future evolution, not a commitment of this version

The Git synchronization model depends on a premise the author confirmed for v1: the two machines are used one at a time, not in parallel on the same project. If that usage pattern changes — for example, an agent running for long stretches on one machine while work continues live on the other — Git synchronization stops being sufficient, because there is no real-time visibility between the two ends. In that scenario the natural evolution returns to a single central instance, reached over authenticated MCP/HTTP from both machines, without synchronizing copies of a database or files.

That phase only starts if the simultaneous usage pattern proves real. It would require its own decisions about network, authentication, concurrency and operating an always-on service — the same design that was considered and discarded here for not matching actual use today.
