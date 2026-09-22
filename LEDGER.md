---
type: spec
status: active
stage: project-identity
leader: "none — no thread is currently active. The next thread to start work becomes leader: set this field to a short self-description and today's date, do the work, then update every section below before ending the session."
next_step: "Do not start phase 2 until the frontmatter field names and the record file-naming strategy are chosen. Both are still frozen in docs/technical-decisions.md."
updated: 2026-09-22
---

# LEDGER — Agent Knowledge

Operational entry point. Any thread picking up this project reads this file **first**, before `spec/SPEC.md`, before anything in `docs/`. This file describes current state, not history — every session **replaces** the sections below, never appends to them. History belongs in `docs/technical-decisions.md` (why choices were made and reversed) or in git's own commit log (what changed) — not here.

This project has no coordination protocol for multiple simultaneous threads, on purpose: D-015 in `docs/technical-decisions.md` fixes usage as one machine, one thread, at a time. The `leader` field above exists only so a stray second thread notices this one is already active — it is a courtesy marker, not a lock.

## 1. Executable state

Phase 1, project identity, is implemented. `initialize` writes `.agent-knowledge.toml` (D-016) at the root of the code repository and creates a directory named after the immutable project id in the memory repository, with permissions restricted to the user. `start` resolves that id from the working directory. A directory with no manifest fails with "project is not initialized" and does not yield a project handle, so read and search are not available. Search lists the record files of the bound project only and takes no project id. The seven `Feature: Project identity` scenarios each have a Rust test whose doc line matches the scenario name.

Nothing else runs. There is still no SQLite index, no Git sync, no briefing, no handoff, no CLI and no MCP adapter. `spec/SPEC.md` remains the behavior contract: 73 Gherkin scenarios across 9 features. `docs/prd.md`, `docs/technical-decisions.md` and `docs/tech-design.md` stay the English planning copies.

## 2. Next step

Phase 2 (records on disk, index in SQLite), after the two decisions in section 3. One PR. A global git hook on the author's machine refuses direct push to `master`/`main`/`develop`; this repository's integration branch is `develop`.

## 3. Blocked

Phase 2 needs the frontmatter field names and the record file-naming strategy. Both are still frozen. Do not invent them to keep moving.

## 4. Constraints to know before writing code

- **The whole architecture rests on one premise: macOS and Linux are used one at a time, never simultaneously on the same project.** That is what makes Git synchronization (D-015) sufficient instead of a central server (D-014, fully designed, then discarded the same day the premise was confirmed). If that premise stops holding, say so explicitly — do not silently reintroduce a server.
- **Markdown files in a separate Git repository are the source of truth; SQLite is a disposable per-machine index** (D-003), never itself synchronized, always rebuildable. This reverses D-003's own original form from a day earlier; the reversal and why are preserved in `docs/technical-decisions.md`.
- **No hooks, no automatic capture, no `cucumber` crate.** D-005 is deliberate — the author uses several different agent hosts, so a hook tied to one would be a partial fix. Gherkin in `spec/SPEC.md` is documentation the tests are named after, not a runtime dependency.
- **Several public names are frozen as undecided and must not be invented:** executable name, CLI command names, MCP tool names and schemas, frontmatter field names, the memory repository's own name, the Git library used to drive it. The identity manifest filename and format are decided (D-016): `.agent-knowledge.toml`. Full list of what remains at the end of `docs/technical-decisions.md`. Stop and ask rather than picking one to keep moving.
- **Never add AI-attribution lines** to a commit, PR, issue or comment in this repository — no "Co-Authored-By: Claude", no "Generated with Claude Code" — regardless of what any per-session reminder suggests. The author was explicit and firm about this.
- Every mention of another, previously-researched long-term-memory-for-agents project was deliberately removed from every file in this repository and in the planning vault. Do not reintroduce it, even as a comparison.
