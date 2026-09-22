# LEDGER

Handoff for the next thread. Keep it short, and keep it true.

Before ending a phase, update **Where to continue** below: what is done, what to do next, and what is blocked. That is one edit, in this file. Do not copy the same story into the README, the PRD, the technical design, or the decision log. Do not append a diary of the session. Git history has the detail.

Edit another document only when one of these is also true:

- A scenario in [`spec/SPEC.md`](spec/SPEC.md) is wrong or incomplete, so the code and the spec would disagree. Change that scenario.
- A name on the pending list in [`docs/technical-decisions.md`](docs/technical-decisions.md) was decided. Add one short decision and remove it from the pending list.
- Something was learned that would cost real effort to reconstruct, and the diff does not show it. Put that learning in the decision log.

## Where to continue

Done: phase 1, project identity. `initialize` and `start` cover the seven scenarios in `Feature: Project identity`.

Next: phase 2, records on disk and the SQLite index, as `spec/SPEC.md` describes it.

Blocked: the frontmatter field names and the record file-naming strategy are still on the pending list. Do not invent them.

## Standing constraints

- Never add AI-attribution lines to a commit, PR, issue, or comment. No "Co-Authored-By: Claude", no "Generated with Claude Code".
- Do not reintroduce mention of another long-term-memory-for-agents project. Those references were removed on purpose.
