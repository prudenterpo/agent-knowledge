# Agent Knowledge

Operational memory for a code repository: what was decided and why, which traps were already hit, which procedures worked, and where the last session stopped.

Agents read and write it over [MCP](https://modelcontextprotocol.io); a human reads and writes it over a CLI. Knowledge is stored as Markdown files in a dedicated Git repository, so it travels between machines through Git itself — there is no server and no always-on process.

## Status

**Foundation only.** No behavior is implemented yet. `spec/SPEC.md` holds the behavior contract and the implementation phases; nothing in this repository claims to work beyond building cleanly.

Product scope: [`docs/prd.md`](docs/prd.md). Why each technical choice was made: [`docs/technical-decisions.md`](docs/technical-decisions.md). Component structure: [`docs/tech-design.md`](docs/tech-design.md).

## Shape of the system

- **Source of truth:** Markdown files with frontmatter, in a separate Git repository, one directory per project id, one file per record.
- **Index:** SQLite + FTS5, local to each machine, rebuilt from those files, never synchronized, never authoritative.
- **Sync:** `git pull` before reads and writes, one atomic commit per write, `push` with explicit reporting when connectivity is missing. A write that cannot be pushed stays committed locally and is reported as unsynchronized, never lost.
- **Conflicts:** expected-revision checks at file granularity. A record changed by the other machine since your last read is rejected as an explicit conflict, never silently overwritten.
- **No LLM, no embeddings, no hooks:** the briefing is assembled by explicit rules; knowledge is registered by explicit calls.

## Build

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

CI runs the same three on Ubuntu and macOS, plus a dependency audit.

## Working on this repository

`spec/SPEC.md` is the source of truth for behavior and how to implement it. Several public names are deliberately undecided — see the pending list at the end of `docs/technical-decisions.md` — and must not be invented while implementing.

Picking this up for the first time, or after a break? Read [`docs/handoff.md`](docs/handoff.md) once — it is a static note about where planning stopped and implementation should start, not an ongoing artifact.
