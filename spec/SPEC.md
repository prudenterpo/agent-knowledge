# Agent Knowledge — behavior specification

Source of truth for **behavior**. If behavior changes, change the scenario here first. If you have not read [`../LEDGER.md`](../LEDGER.md) yet, read that first instead — it says which phase is current.

- Why each technical choice was made, including every `D-xxx` reference below: [technical decisions](../docs/technical-decisions.md).
- Product scope, users and non-goals: [product requirements](../docs/prd.md).
- Component structure and data model: [technical design](../docs/tech-design.md).

## How to implement with an AI agent

1. Take the next phase below. Do not skip ahead.
2. For each scenario in that phase, write a Rust test whose name (or `///` doc line) matches the scenario name exactly.
3. Make it pass. **Do not add the `cucumber` crate**: Gherkin is the spec, `cargo test` is the runner. Every dependency needs justification (D-001, D-010).
4. One commit per phase. Do not add task files, status docs or process scaffolding to "organize the work".
5. If a scenario turns out to be wrong or impossible, change the scenario here and say so — never leave code and spec disagreeing.

Names still frozen as undecided (crate name, executable name, CLI command names, MCP tool names, frontmatter field names) must not be invented while implementing. The identity manifest filename and format are decided in D-016. See the pending list in the decision log.

## When knowledge should be registered

The system does not decide this — the agent does, guided by project instructions (D-005). This criterion is not a testable scenario; it belongs in the instructions shipped to each project:

> Register when the information would cost real effort to reconstruct later: a decision and the reason another option was rejected; a failure or limitation that cost time to discover; a procedure that worked and has been needed more than once. Do not register task chatter, restatements of code that the repository already shows, or anything that would be obvious from reading the diff.

The categories in `Feature: Knowledge records` (decision, gotcha, procedure, note) exist to make that judgement explicit at write time — choosing a category is what forces the agent to ask whether the record is durable at all.

## Phases

Each phase lists the scenarios that must be green before moving on. Order matters: each phase assumes the previous.

### 0. Skeleton

Cargo package, reproducible build on macOS and Linux, formatting, lint without warnings, test harness, dependency audit wired in from the first commit (D-010).

Splitting into workspace members waits until the executable and CLI names are decided, so the foundation ships a library target only.

No scenarios. Done when `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets` and the dependency audit all run clean on both platforms.

### 1. Project identity

Manifest in the code repository, immutable project id, resolution from the working directory, per-project isolation.

Scenarios: all of `Feature: Project identity`.

### 2. Records on disk, index in SQLite

Markdown files with frontmatter as the source of truth; SQLite + FTS5 built from them; create, update, obsolete, supersede. Expected revision **within one machine** only — no Git yet.

Scenarios: `Feature: Knowledge records`, plus `Feature: Search` except the rebuild scenario.

### 3. Index rebuild

The index is disposable and reconstructible at any time (D-003, D-009).

Scenarios: `Search: index rebuilt from files returns the same results`, `Search: corrupted index is rebuilt, not repaired`.

### 4. Git synchronization

Pull before read and before write, atomic local commit, push with explicit failure reporting, conflict detection across machines.

Scenarios: all of `Feature: Git synchronization`.

### 5. Briefing and handoff

Deterministic briefing with limits and quotas; one active handoff per project.

Scenarios: `Feature: Briefing`, `Feature: Handoff`.

### 6. CLI

Human operations over the same core: init, inspect, search, create, update, obsolete, force sync, rebuild index. Distinct exit codes per error category.

Scenarios: all of `Feature: CLI`.

### 7. MCP adapter

stdio transport over the already-validated core.

Scenarios: all of `Feature: MCP protocol`.

### 8. Real agents and pilot

Integration with Codex and Claude Code; pilot in at least two real repositories, switching machines mid-task.

Scenarios: `Feature: Cross-machine continuity`.

## Quick map

| Feature | Area of code it should touch |
|---|---|
| Project identity | manifest parsing, identity resolution, memory-repo directory layout |
| Knowledge records | record store (file read/write), frontmatter parser, revision handling |
| Search | index builder, FTS5 query layer |
| Briefing | briefing assembler, ordering and quota rules |
| Handoff | record store, active/inactive transition |
| Git synchronization | sync layer (pull/commit/push), conflict detection |
| CLI | CLI adapter, exit code mapping |
| MCP protocol | MCP adapter, input validation, stdout/stderr discipline |
| Cross-machine continuity | end-to-end, no single module |

---

## Feature: Project identity

```gherkin
Feature: Project identity
  As a developer using several repositories
  I want a project to keep its memory regardless of path or machine
  So that moving or re-cloning a repository does not lose or mix knowledge

  Scenario: Initializing a project writes an immutable manifest
    Given a repository with no memory manifest
    When I initialize the project
    Then a manifest containing an immutable project id is written atomically
    And a directory named after that id exists in the memory repository

  Scenario: Initializing twice keeps the original id
    Given a repository already initialized
    When I initialize the project again
    Then the original project id is unchanged
    And no second directory is created in the memory repository

  Scenario: An invalid manifest is never overwritten
    Given a repository whose manifest exists but is malformed
    When I initialize the project
    Then the operation fails with an actionable error
    And the existing manifest file is left untouched

  Scenario: Identity survives a path change
    Given an initialized project with recorded knowledge
    When the repository is moved to a different absolute path
    Then the same project id resolves
    And the same records are available

  Scenario: Identity survives a fresh checkout that keeps the manifest
    Given an initialized project with recorded knowledge
    When the repository is cloned to another directory with the manifest preserved
    Then the same project id resolves
    And the same records are available

  Scenario: An uninitialized project exposes no normal tools
    Given a repository with no manifest
    When a client starts in that directory
    Then read and write tools for knowledge are not available
    And the reported error says the project is not initialized

  Scenario: Two projects cannot read each other
    Given two initialized projects each with their own records
    When I search from within the first project
    Then only records of the first project are returned
    And no tool of normal use accepts another project as a parameter
```

## Feature: Knowledge records

```gherkin
Feature: Knowledge records
  As an agent working on a project
  I want to record durable knowledge with explicit state
  So that a later session can trust what it reads and know what is stale

  Scenario: Creating a record writes a Markdown file with frontmatter
    Given an initialized project
    When I create a record with a title, a category and Markdown content
    Then a Markdown file is written in that project's directory
    And its frontmatter carries an immutable id, category, state active, priority, revision and timestamps
    And the file body is the Markdown content unchanged

  Scenario: The stored Markdown is not rewritten by the system
    Given Markdown content containing lists, code fences and wiki links
    When I create a record with that content
    Then the file body is byte-identical to the submitted content

  Scenario: A record without a required field is rejected
    Given an initialized project
    When I create a record missing the title or the category
    Then the operation fails with a structured validation error
    And no file is written

  Scenario: An unknown category is rejected
    When I create a record with a category other than decision, gotcha, procedure or note
    Then the operation fails with a structured validation error

  Scenario: Creating a record updates the local index in the same operation
    When I create a record
    Then the record is findable by search without any further command
    And file, index and commit were all produced, or none of them was

  Scenario: Updating a record requires the expected revision
    Given an existing record at revision N
    When I update it declaring expected revision N
    Then the update is applied
    And the stored revision becomes N+1

  Scenario: Updating with a stale revision is a conflict
    Given an existing record at revision N
    When I update it declaring expected revision N-1
    Then the operation fails with an explicit revision conflict
    And the stored record is unchanged

  Scenario: Changing state, priority or supersession bumps the revision
    Given an existing record at revision N
    When I change only its priority
    Then the stored revision becomes N+1

  Scenario: Marking a record obsolete keeps it readable
    Given an active record
    When I mark it obsolete
    Then its state is obsolete
    And the file still exists with its content intact

  Scenario: Superseding links two records of the same project
    Given an active record A and a new record B in the same project
    When I record that B supersedes A
    Then A carries a supersession relation pointing at B
    And A is marked obsolete

  Scenario: Superseding across projects is refused
    Given record A in one project and record B in another
    When I record that B supersedes A
    Then the operation fails with a validation error

  Scenario: Nothing is ever deleted automatically
    Given records of every category, some obsolete for a long time
    When any normal operation runs
    Then no record file is removed

  Scenario: A frontmatter format newer than supported is refused per file
    Given a record whose frontmatter declares a format version newer than the client supports
    When I read or index the project's records
    Then that specific file is refused with an actionable error
    And every other record in the project is still readable
```

## Feature: Search

```gherkin
Feature: Search
  As an agent about to make a decision
  I want to find existing knowledge by plain words
  So that I do not repeat a decision or rediscover a known trap

  Scenario: Plain text input needs no FTS5 syntax
    Given records mentioning "connection pool"
    When I search for: connection pool
    Then matching records are returned
    And the caller never had to write FTS5 operators

  Scenario: Special syntax characters are treated literally
    When I search for a string containing quotes, asterisks, parentheses or a NEAR operator
    Then the search does not error
    And the input is matched as literal text

  Scenario: Search is limited to the current project
    Given two projects with records sharing the same words
    When I search from within one project
    Then only that project's records are returned

  Scenario: Results are ordered by textual relevance first
    Given one record whose title and body match the query strongly and another matching weakly
    When I search
    Then the strong match ranks above the weak one

  Scenario: Priority only adjusts, never overrides relevance
    Given a high-priority record with no textual match and a normal-priority record that matches
    When I search
    Then the matching record is returned
    And the non-matching high-priority record is not promoted into the results

  Scenario: Ties break deterministically
    Given two records with identical relevance and priority
    When I search the same query twice
    Then both runs return them in the same order

  Scenario: Obsolete records are excluded unless requested
    Given one active and one obsolete record matching the query
    When I search without asking for obsolete records
    Then only the active one is returned
    When I search asking for obsolete records
    Then both are returned

  Scenario: Result set is bounded and reports truncation
    Given more matching records than the configured result limit
    When I search
    Then at most the limit is returned
    And the response states that more results exist

  Scenario: Index rebuilt from files returns the same results
    Given a project with records and a query with known results
    When the local index is deleted and rebuilt from the Markdown files
    Then the same query returns the same records in the same order

  Scenario: Corrupted index is rebuilt, not repaired
    Given a local index file that fails an integrity check
    When a client starts
    Then the index is discarded and rebuilt from the Markdown files
    And no partial repair is attempted
```

## Feature: Briefing

```gherkin
Feature: Briefing
  As an agent starting a task
  I want a short, predictable briefing
  So that I begin with the project's real constraints without burning context

  Scenario: Briefing carries pinned content, active priorities and the handoff
    Given pinned records, high-priority active decisions and gotchas, and an active handoff
    When I request the briefing
    Then all four kinds appear in the briefing

  Scenario: Obsolete records never appear in the briefing
    Given an obsolete record with high priority and the pinned flag set
    When I request the briefing
    Then that record does not appear
    And it is still reachable by search

  Scenario: Briefing respects its total limit
    Given more eligible content than the configured total limit
    When I request the briefing
    Then the rendered briefing does not exceed the limit

  Scenario: Each section respects its quota
    Given more high-priority decisions than the decisions quota allows
    When I request the briefing
    Then the decisions section contains at most its quota
    And the gotchas section is not starved by the excess decisions

  Scenario: Overflow stays reachable by search
    Given content dropped from the briefing because of the limit
    When I search for that content
    Then it is returned

  Scenario: Briefing is byte-for-byte deterministic
    Given an unchanged set of synchronized files and unchanged limits
    When I request the briefing twice
    Then both renderings are byte-identical

  Scenario: Briefing never calls an LLM
    When I request the briefing with no network available
    Then it is produced normally
```

## Feature: Handoff

```gherkin
Feature: Handoff
  As a developer resuming work
  I want the last handoff to be the one I get
  So that I know where the previous session stopped without reading history

  Scenario: Writing a handoff makes it the only active one
    Given a project with an active handoff
    When I write a new handoff
    Then the new one is active
    And the previous one is inactive

  Scenario: The previous handoff stays recoverable
    Given a handoff that has just been superseded
    When I ask for previous handoffs
    Then the superseded one is returned with its content intact

  Scenario: The active handoff appears in the next briefing
    Given a handoff written at the end of a session
    When a new session requests the briefing
    Then the handoff content appears in it

  Scenario: A handoff carries the four scoped fields
    When I write a handoff
    Then it stores completed work, pending work, recent decisions or limitations, and a concrete next step

  Scenario: Updating a handoff requires the expected revision
    Given an active handoff at revision N
    When I write a new handoff declaring expected revision N-1
    Then the operation fails with an explicit revision conflict
    And the active handoff is unchanged

  Scenario: No failure leaves two active handoffs
    Given a project with an active handoff
    When writing a new handoff fails partway
    Then exactly one handoff is active
    And the repository has no commit containing two active handoffs
```

## Feature: Git synchronization

```gherkin
Feature: Git synchronization
  As a developer using macOS and Linux one at a time
  I want memory to travel between machines through Git
  So that continuity does not depend on running a server

  Scenario: First run clones the memory repository
    Given a machine with no local copy of the memory repository
    When a client starts
    Then the memory repository is cloned locally
    And the local index is built from it

  Scenario: A write commits atomically without network
    Given no connectivity to the Git remote
    When I create a record
    Then the file is written, the index updated and a commit created
    And the response states that the write is local and not yet synchronized

  Scenario: Push failure never loses the local commit
    Given a write whose push failed for lack of connectivity
    When connectivity returns and I synchronize
    Then the pending commit is pushed
    And the record becomes visible to the other machine

  Scenario: One write produces one commit
    When I create a record
    Then exactly one commit is added to the memory repository
    And its message follows the standard format

  Scenario: Pull happens before a write
    Given the other machine has pushed a change to a different record
    When I create a record here
    Then the remote change is pulled before my commit is created

  Scenario: A change by the other machine to the same record is a conflict
    Given record R at revision N locally
    And the other machine has pushed R at revision N+1
    When I update R declaring expected revision N
    Then the operation fails with an explicit revision conflict
    And my working copy is left clean, with no partial write

  Scenario: A rejected push surfaces as a conflict, not a generic error
    Given a local commit created against a stale remote state
    When the push is rejected
    Then the error names the diverging record
    And it is reported as a conflict, distinct from loss of connectivity

  Scenario: History is never rewritten
    When any operation synchronizes
    Then no force push is performed
    And no existing commit is amended or dropped

  Scenario: Remote unavailable still allows reading
    Given no connectivity to the Git remote
    When I request a briefing or run a search
    Then the last locally known state answers the request
    And the response signals that the local state may be stale

  Scenario: The SQLite index never travels through Git
    When any operation writes to the memory repository
    Then no SQLite file is added, committed or pushed

  Scenario: Local clone and index are created with restricted permissions
    When the memory repository is cloned or the local index is created
    Then neither is readable by another user on the machine
```

## Feature: CLI

```gherkin
Feature: CLI
  As a developer inspecting or driving memory by hand
  I want predictable exit codes
  So that scripts and my own judgment can tell success from each kind of failure apart

  Scenario: Success exits zero
    Given a well-formed command that completes without error
    When it runs
    Then the process exits with the code for success

  Scenario: Invalid input exits with its own code
    Given a command with missing or malformed arguments
    When it runs
    Then the process exits with the code for invalid input
    And the reason is printed before exiting

  Scenario: A revision conflict exits with its own code
    Given a write whose declared revision no longer matches the record
    When it runs
    Then the process exits with the code for conflict
    And the exit code is distinct from invalid input and from internal failure

  Scenario: Failure to synchronize exits with its own code
    Given no connectivity to the Git remote
    When a command that needs synchronization runs
    Then the process exits with the code for synchronization unavailable
    And that code is distinct from a revision conflict

  Scenario: An internal failure exits with its own code
    Given a local persistence failure, such as a full disk
    When a command hits it
    Then the process exits with the code for internal failure
    And the message does not expose internal library detail

  Scenario: Forcing synchronization is available as its own operation
    Given records written locally but not yet pushed
    When I ask the CLI to synchronize
    Then it pulls, resolves what it can, and pushes
    And it reports what, if anything, is still unsynchronized

  Scenario: Rebuilding the index is available as its own operation
    Given a local index believed to be stale or corrupted
    When I ask the CLI to rebuild it
    Then it is deleted and rebuilt from the memory repository's files
    And no record content is altered by that operation
```

## Feature: MCP protocol

```gherkin
Feature: MCP protocol
  As an MCP-compatible agent
  I want a well-behaved stdio server
  So that a malformed call degrades into an error instead of a broken session

  Scenario: Handshake announces the supported tools
    Given an initialized project
    When a host performs the MCP handshake
    Then the server announces its tool set

  Scenario: Tools are announced only for an initialized project
    Given a directory with no manifest
    When a host performs the MCP handshake
    Then knowledge tools are not announced
    And the reason is reported as project not initialized

  Scenario: A malformed message returns a structured error
    When the host sends a message that is not valid JSON-RPC
    Then a structured protocol error is returned
    And the process stays alive and able to serve the next request

  Scenario: An oversized input is refused before processing
    When the host sends an input above the configured size limit
    Then a structured error is returned naming the limit
    And no partial record is written

  Scenario: stdout carries protocol only
    Given a session that triggers warnings and errors internally
    When the session runs to completion
    Then every byte written to stdout is protocol traffic
    And all diagnostics went to stderr

  Scenario: No secret or record content leaks into diagnostics
    Given a failing operation on a record with sensitive content
    When the error is reported
    Then the diagnostic output contains neither credentials nor the record body

  Scenario: End of input shuts down cleanly
    When the host closes stdin
    Then the process exits without error
    And no lock or temporary file is left behind

  Scenario: Requests are served one at a time
    Given two requests arriving back to back
    When the server processes them
    Then they are handled sequentially, with no interleaved writes
```

## Feature: Cross-machine continuity

```gherkin
Feature: Cross-machine continuity
  As a developer switching between macOS and Linux
  I want work to resume where it stopped
  So that changing machine costs nothing in context

  Scenario: A decision written on macOS is found on Linux
    Given an agent on macOS recorded a decision and synchronized
    When an agent on Linux synchronizes and searches for it
    Then the decision is returned with its content intact

  Scenario: A handoff written on Linux drives the next macOS session
    Given an agent on Linux wrote a handoff and synchronized
    When an agent on macOS starts a session and requests the briefing
    Then the handoff appears in the briefing

  Scenario: Two agents on the same machine share the same state
    Given Codex and Claude Code configured on the same machine and project
    When one records a decision and the other requests a briefing
    Then both operate on the same records

  Scenario: The same briefing is produced on both machines
    Given both machines synchronized to the same commit
    When each requests the briefing for the same project
    Then both renderings are byte-identical
```

## Done when

- Every scenario above has a matching Rust test and it passes on macOS and on Linux.
- `cargo fmt --check`, `cargo clippy` without warnings, `cargo test` and the dependency audit are green on both platforms.
- The frontmatter parser and the MCP input path have property tests or fuzzing before their interfaces are declared stable (D-010).
- A pilot ran in at least two real repositories, switching machines mid-task, and the second machine recovered decisions and the stopping point without the knowledge being re-explained by hand.
- No document in this project claims behavior that has no scenario here.
