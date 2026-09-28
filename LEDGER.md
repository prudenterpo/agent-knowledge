# LEDGER

Handoff for the next thread. Keep it short, and keep it true.

Before ending a phase, update **Where to continue** below: what is done, what to do next, and what is blocked. That is one edit, in this file. Do not copy the same story into the README, the PRD, the technical design, or the decision log. Do not append a diary of the session. Git history has the detail.

Edit another document only when one of these is also true:

- A scenario in [`spec/SPEC.md`](spec/SPEC.md) is wrong or incomplete, so the code and the spec would disagree. Change that scenario.
- A name on the pending list in [`docs/technical-decisions.md`](docs/technical-decisions.md) was decided. Add one short decision and remove it from the pending list.
- Something was learned that would cost real effort to reconstruct, and the diff does not show it. Put that learning in the decision log.

## Where to continue

Done: phase 1, project identity. initialize and start cover the seven scenarios in Feature: Project identity. The v1 contract is frozen in D-017, the engineering constitution is present, and the behavior specification has been reconciled with the Git and SQLite recovery model.

Next: one leader owns the feature/agent-knowledge-v1 worktree and delivers phases 2 through 7 as one BDD-driven pull request. Read this file, CONSTITUTION.md, docs/technical-decisions.md, docs/tech-design.md and spec/SPEC.md before editing. First verify the current branch, working tree and phase-1 tests. Then implement one phase at a time, add the matching Rust tests, run the phase quality gates, and keep one conventional English commit per phase. Do not start the real-repository pilot in phase 8 without explicit authorization.

Blocked: none for phases 2 through 7. Phase 8 is intentionally deferred because it changes real repository and agent configuration.

## Standing constraints

- Never add AI-attribution lines to a commit, PR, issue, or comment. No "Co-Authored-By: Claude", no "Generated with Claude Code".
- Source and memory-repository commits use type(scope): English summary. No Codex-, Claude- or agent-branded branch, commit, pull-request, issue or comment text is allowed.
- Do not reintroduce mention of another long-term-memory-for-agents project. Those references were removed on purpose.
