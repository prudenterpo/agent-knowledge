# Working on Agent Knowledge

## The spec is the source of truth for behavior

`spec/SPEC.md` holds the phases and the Gherkin scenarios. Take the next phase, write a Rust test whose name matches the scenario name exactly, make it pass. If a scenario turns out to be wrong or impossible, **change the scenario first and say so** — never leave code and spec disagreeing.

Do not add the `cucumber` crate. Gherkin is the spec, `cargo test` is the runner. Every dependency needs a justification the maintainer can explain.

Do not add task files, status documents or process scaffolding to "organize the work". One commit per phase.

## Names that are deliberately undecided

Several public names are frozen as open decisions and **must not be invented while implementing**. If the work needs one, stop and ask:

- executable name and CLI command names;
- MCP tool names and schemas;
- the identity manifest's filename and format;
- frontmatter field names and their serialization;
- the memory repository's name;
- exact input, result and briefing limits;
- the Git library used (shelling out to `git` vs. a crate such as `git2`).

## Everything that lands in git is English

Chat language does not set artifact language. Conversation may be in Portuguese. Code, comments, documentation, commit messages, PR titles and descriptions, and issue text are English — no exceptions.

Commits follow `type(scope): short summary`:

- `type`: `feat` `fix` `test` `docs` `refactor` `chore` `build`
- `scope`: the area (`identity`, `records`, `search`, `briefing`, `handoff`, `sync`, `mcp`, `spec`)
- Subject: imperative, lowercase after the colon, no period, ≤ 72 chars
- Body (optional): why, not how
- No agent signature, no co-author trailer, no generated-by line

## Constraints that are not negotiable without changing a decision

- The domain and persistence core stays in safe Rust (`unsafe_code = "deny"`). Unsafe is allowed only in a small, justified, tested FFI boundary that opts in explicitly.
- The SQLite index is derived and disposable. It is never the source of truth, never synchronized, and never repaired — only rebuilt.
- In MCP mode, stdout carries protocol traffic and nothing else. Diagnostics go to stderr, without secrets or record content.
- No force push and no history rewriting of the memory repository.
- Nothing is ever deleted automatically. Knowledge that stops being true is marked obsolete or superseded.
