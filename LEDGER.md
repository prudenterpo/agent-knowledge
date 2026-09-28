# LEDGER

Handoff for the next thread. Keep it short, and keep it true.

Before ending a phase, update **Where to continue** below: what is done, what to do next, and what is blocked. That is one edit, in this file. Do not copy the same story into the README, the PRD, the technical design, or the decision log. Do not append a diary of the session. Git history has the detail.

Edit another document only when one of these is also true:

- A scenario in [`spec/SPEC.md`](spec/SPEC.md) is wrong or incomplete, so the code and the spec would disagree. Change that scenario.
- A name on the pending list in [`docs/technical-decisions.md`](docs/technical-decisions.md) was decided. Add one short decision and remove it from the pending list.
- Something was learned that would cost real effort to reconstruct, and the diff does not show it. Put that learning in the decision log.

## Where to continue

Done: phases 1 through 7 are implemented on feature/agent-knowledge-v1. The application core now orchestrates Git pull, durable file write, index update, record or handoff commit, and push; supersession persists the replacement before obsoleting its predecessor. MCP exposes structured tool contracts and all frozen update fields; CLI supports active-handoff revision input. Local Linux evidence: cargo fmt --all --check, cargo clippy --all-targets -- -D warnings, cargo test --all-targets (75 tests), and cargo doc --no-deps passed.

Next: wait for the final GitHub CI run on Linux x86_64, macOS aarch64 and dependency audit; then take PR #5 out of draft for Rodrigo's review. Do not merge it. Do not start phase 8 without explicit authorization.

Blocked: phase 8 is intentionally deferred because it changes real repository and agent configuration.

## Standing constraints

- Never add AI-attribution lines to a commit, PR, issue, or comment. No "Co-Authored-By: Claude", no "Generated with Claude Code".
- Source and memory-repository commits use type(scope): English summary. No Codex-, Claude- or agent-branded branch, commit, pull-request, issue or comment text is allowed.
- Do not reintroduce mention of another long-term-memory-for-agents project. Those references were removed on purpose.
