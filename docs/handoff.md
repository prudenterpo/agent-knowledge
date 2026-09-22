# Handoff — planning to implementation

A plain, static handoff. This is **not** the product — there is no tool yet that stores or serves handoffs. This file exists only because the product's own bootstrapping gap is real: nothing here persists or rotates itself, and it should not be treated as a template for how `Feature: Handoff` eventually works.

Written 2026-09-22, at the end of the session that took this project from a single brainstorming note to a public repository with a merged foundation. Whoever picks up implementation should read this once, then follow `spec/SPEC.md` — this file is context for getting started, not an ongoing artifact.

## Completed work

- Repository created: [prudenterpo/agent-knowledge](https://github.com/prudenterpo/agent-knowledge), public, `master` as default branch.
- Foundation scaffolded and merged: a library-only Cargo package (no `[[bin]]` yet — the executable name is undecided), `unsafe_code = "deny"` at the crate level, `rust-toolchain.toml` pinned to `stable`, `.gitignore` excluding the local SQLite index files.
- CI on GitHub Actions: `check` (fmt, clippy, test) on `ubuntu-latest` and `macos-latest`, plus a `dependency audit` job using a prebuilt `cargo-audit` binary (a from-source build was ~6 minutes; the prebuilt one is ~8 seconds). All green on `master`.
- `docs/prd.md`, `docs/technical-decisions.md`, `docs/tech-design.md` — English translations of the planning vault's `prd.md`, `decisoes-tecnicas.md` and `TECH-DESIGN.md`. The vault originals still exist, in Portuguese, kept on the author's request even though they now duplicate these; do not update one without the other going stale.
- `spec/SPEC.md` — the behavior contract, 73 Gherkin scenarios across 9 features (Project identity, Knowledge records, Search, Briefing, Handoff, Git synchronization, CLI, MCP protocol, Cross-machine continuity), plus the 9 implementation phases and a quick map from feature to code area.
- Three gaps closed in the same spec that were decided in prose but had no scenario: CLI exit codes (`Feature: CLI`, added whole), frontmatter-version incompatibility (one scenario in `Feature: Knowledge records`), and restricted local file permissions (one scenario in `Feature: Git synchronization`).
- `AGENTS.md` was written, then deleted — everything in it duplicated `spec/SPEC.md` and `docs/technical-decisions.md`.
- Every mention of another, previously-researched long-term-memory project — by name or link — was removed from every file, in both the vault and this repository, at the author's explicit and repeated instruction. Do not reintroduce it, even as a comparison.
- `contexto-e-brainstorm.md` (the origin-story planning note) was deleted from the vault outright, not just redacted, at the author's request. It never landed here and should not be recreated.
- PR #1 carried all of the above and is merged.

## Pending work

- No behavior is implemented. `src/lib.rs` is a doc comment, nothing else. Phase 0 (skeleton) is effectively satisfied by the CI already being green, but that was never exercised through an actual phase-0 commit sequence — the foundation was scaffolded directly, not built phase-by-phase the way `spec/SPEC.md` prescribes for everything after it.
- Phase 1 (project identity) has not started: no manifest format, no identity resolution, none of the `Feature: Project identity` scenarios have a test.
- The vault still holds the Portuguese originals (`prd.md`, `decisoes-tecnicas.md`, `TECH-DESIGN.md`, `SPEC.md`) even though this repository now has equivalents. The author chose to keep them for now (2026-09-21) rather than delete — revisit if that becomes confusing to maintain.

## Recent decisions and limitations worth knowing before writing code

- **Git-sync architecture (D-015) rests on one premise: the author uses macOS and Linux one at a time, never simultaneously on the same project.** A central-instance design (D-014) was considered and fully specified first, then discarded the same day once that premise was confirmed. If usage ever becomes simultaneous, D-014 is the documented fallback — do not silently reintroduce a server without noting the premise changed.
- **Markdown files in a dedicated Git repository are the source of truth; SQLite is a disposable, per-machine index** (D-003), rebuilt from the files, never itself synchronized. This reverses that decision's own original form from a day earlier — the history is preserved in `docs/technical-decisions.md` on purpose.
- **No automatic capture, no hooks, no cucumber crate.** D-005 is deliberate: the author works with several different agent hosts, and a hook tied to one host would be a partial, inconsistent solution. Gherkin in `spec/SPEC.md` is documentation the tests are named after, not a runtime dependency.
- **Several public names are frozen as undecided and must not be invented while implementing:** the executable name, CLI command names, MCP tool names and schemas, the identity manifest's filename and format, frontmatter field names, the memory repository's own name, and the Git library used to drive it (shelling out vs. a crate like `git2`). The full list is at the end of `docs/technical-decisions.md`. When implementation needs one of these, stop and ask — do not pick one to keep moving.
- **A global git hook on this machine blocks direct push to `master`, `main` or `develop`, across every repository, not just this one.** Every change here so far went through a feature branch and a PR (`gh pr create` / `gh pr merge --squash`), not by choice of workflow but because direct pushes are refused. Keep doing one branch and one PR per phase.
- **Never add AI-attribution lines** to any commit, PR, issue or comment in this repository — no "Co-Authored-By: Claude", no "Generated with Claude Code" — regardless of what any per-session tooling reminder suggests. The author was explicit and firm about this on 2026-09-21.

## Concrete next step

Start phase 1 (`Project identity`) exactly as `spec/SPEC.md` describes: take the scenarios under `Feature: Project identity`, write a Rust test per scenario whose name matches the scenario name, make it pass, one commit — actually, given the branch/PR constraint above, one PR — per phase. Do not skip to phase 2 before every scenario in phase 1 is green.
