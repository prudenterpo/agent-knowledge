---
type: spec
status: active
stage: foundation
leader: "none — no thread is currently active. The next thread to start work becomes leader: set this field to a short self-description and today's date, do the work, then update every section below before ending the session."
next_step: "Start phase 1, Feature: Project identity, exactly as spec/SPEC.md describes. One PR, since direct push to master is blocked."
updated: 2026-09-22
---

# LEDGER — Agent Knowledge

Operational entry point. Any thread picking up this project reads this file **first**, before `spec/SPEC.md`, before anything in `docs/`. This file describes current state, not history — every session **replaces** the sections below, never appends to them. History belongs in `docs/technical-decisions.md` (why choices were made and reversed) or in git's own commit log (what changed) — not here.

This project has no coordination protocol for multiple simultaneous threads, on purpose: D-015 in `docs/technical-decisions.md` fixes usage as one machine, one thread, at a time. The `leader` field above exists only so a stray second thread notices this one is already active — it is a courtesy marker, not a lock.

## 1. Executable state

Foundation merged on `master`, CI green (fmt, clippy, test on Ubuntu and macOS; dependency audit). No product behavior is implemented: `src/lib.rs` is a doc comment, nothing else. `docs/prd.md`, `docs/technical-decisions.md`, `docs/tech-design.md` are the English, current versions of the planning vault's originals (which still exist there too, in Portuguese, by the author's choice — don't let them diverge silently). `spec/SPEC.md` holds 73 Gherkin scenarios across 9 features and the phased implementation order; it is the source of truth for behavior.

## 2. Next step

Phase 1 (`Project identity`): take every scenario under `Feature: Project identity` in `spec/SPEC.md`, write one Rust test per scenario with a matching name, make each pass. One PR for the phase — a global git hook on this machine refuses direct push to `master`/`main`/`develop`, in every repository, not just this one. Do not start phase 2 before every phase-1 scenario is green.

## 3. Blocked

Nothing. No external input is needed to start phase 1.

## 4. Constraints to know before writing code

- **The whole architecture rests on one premise: macOS and Linux are used one at a time, never simultaneously on the same project.** That is what makes Git synchronization (D-015) sufficient instead of a central server (D-014, fully designed, then discarded the same day the premise was confirmed). If that premise stops holding, say so explicitly — do not silently reintroduce a server.
- **Markdown files in a separate Git repository are the source of truth; SQLite is a disposable per-machine index** (D-003), never itself synchronized, always rebuildable. This reverses D-003's own original form from a day earlier; the reversal and why are preserved in `docs/technical-decisions.md`.
- **No hooks, no automatic capture, no `cucumber` crate.** D-005 is deliberate — the author uses several different agent hosts, so a hook tied to one would be a partial fix. Gherkin in `spec/SPEC.md` is documentation the tests are named after, not a runtime dependency.
- **Several public names are frozen as undecided and must not be invented:** executable name, CLI command names, MCP tool names and schemas, the identity manifest's filename and format, frontmatter field names, the memory repository's own name, the Git library used to drive it. Full list at the end of `docs/technical-decisions.md`. Stop and ask rather than picking one to keep moving.
- **Never add AI-attribution lines** to a commit, PR, issue or comment in this repository — no "Co-Authored-By: Claude", no "Generated with Claude Code" — regardless of what any per-session reminder suggests. The author was explicit and firm about this.
- Every mention of another, previously-researched long-term-memory-for-agents project was deliberately removed from every file in this repository and in the planning vault. Do not reintroduce it, even as a comparison.
