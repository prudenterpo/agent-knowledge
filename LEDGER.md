# LEDGER

Handoff for the next thread. Keep it short, and keep it true.

Before ending a phase, update **Where to continue** below: what is done, what to do next, and what is blocked. That is one edit, in this file. Do not copy the same story into the README, the PRD, the technical design, or the decision log. Do not append a diary of the session. Git history has the detail.

Edit another document only when one of these is also true:

- A scenario in [`spec/SPEC.md`](spec/SPEC.md) is wrong or incomplete, so the code and the spec would disagree. Change that scenario.
- A name on the pending list in [`docs/technical-decisions.md`](docs/technical-decisions.md) was decided. Add one short decision and remove it from the pending list.
- Something was learned that would cost real effort to reconstruct, and the diff does not show it. Put that learning in the decision log.

## Where to continue

Done: the feature/agent-knowledge-v1 branch contains the phase-2-to-7 core and adapter implementation. Local Linux evidence: cargo fmt --all --check, cargo clippy --all-targets -- -D warnings, cargo test --all-targets (40 tests), and cargo doc --no-deps passed. GitHub CI passed Linux x86_64, macOS aarch64 (macos-14), and dependency audit.

Next: complete the remaining one-test-per-scenario coverage before requesting review: record interruption/newer-schema and all search ordering/bounds scenarios; the remaining Git synchronization scenarios and automatic write-to-commit coupling; briefing limits/quotas; handoff failure/revision scenarios; CLI exit-code/operation scenarios; and MCP initialization, size, diagnostics, and serialization scenarios. Do not start the real-repository pilot in phase 8 without explicit authorization.

Blocked: phase 8 is intentionally deferred because it changes real repository and agent configuration. The current PR is not ready to merge until the listed BDD coverage is completed.

## Standing constraints

- Never add AI-attribution lines to a commit, PR, issue, or comment. No "Co-Authored-By: Claude", no "Generated with Claude Code".
- Source and memory-repository commits use type(scope): English summary. No Codex-, Claude- or agent-branded branch, commit, pull-request, issue or comment text is allowed.
- Do not reintroduce mention of another long-term-memory-for-agents project. Those references were removed on purpose.
