# Technical decisions — Agent Knowledge

This document records the decisions taken for the first version. It exists so that a future agent does not treat as an implicit requirement something that was a conscious choice. When a decision changes, the earlier record is marked as superseded rather than deleted — so that the next revision does not reopen an already-walked path without knowing why.

This document holds **decisions only** — the "why". What the product is and who it serves: [product requirements](./prd.md). What behavior is expected, as testable scenarios, and in what order to build it: [the behavior specification](../spec/SPEC.md). Which components exist and what shape the data has: [technical design](./tech-design.md). Earlier reasoning and discarded paths: [context and brainstorming](./context-and-brainstorming.md).

## Principles governing these decisions

1. Solve real continuity of context across agents, repositories and machines, deliberately kept small.
2. Prioritize usefulness, reliability, integrity and maintainability over the learning goal.
3. Learn Rust and systems fundamentals by building the product that is needed, without adding mechanisms just to exercise the language.
4. Remove optional automation and intelligence before removing durability, search, compatibility or data integrity.
5. Do not introduce infrastructure of our own — server, network, authentication — when a mechanism the author already operates and trusts (Git) solves the same problem. Continuity between macOS and Linux is a real need from v1, but that justifies reusing Git (D-015), not building and hosting a service.
6. Have a single source of truth for each piece of data.

## D-001 — Implementation in Rust for macOS and Linux

**Decision:** implement the first version in stable Rust, using portable POSIX APIs where needed, with a native executable per platform.

**Reason:** Rust delivers a binary close to the system, without a garbage collector, and makes ownership, resource lifetime and concurrency part of a contract verified at compile time. That allows studying memory, files, processes, FFI, SQLite and IPC while building a tool reliable enough for professional use. Performance is not the main reason; SQLite, disk and IPC will be the likelier limits.

**Consequences:** the project must model ownership and errors clearly, bound buffers, handle malformed input, and use mature dependencies for problems that are not the product's differentiator. The core stays in safe Rust; unsafe Rust is exceptional and encapsulated. That removes entire classes of memory failure without removing the need for integration tests, correct transactions and security review.

**Alternatives rejected for now:**

- **C**: offers direct contact with manual allocation and ABI, but transfers memory risks into a professional tool without improving its value proposition.
- **Go**: excellent distribution, but with a garbage collector and less exposure to manual resource management.
- **Python**: fast to prototype, but would require a runtime or extra packaging for the local CLI experience.
- **Java**: reliable and familiar, but a JVM is a heavier operational dependency than a short-lived local MCP process needs.
- **C++**: possible, but adds a larger language and ecosystem without improving the balance between professional tool and systems learning.

## D-002 — MCP over stdio, no daemon

**Decision:** the local binary acts as an MCP server over stdio. The agent host starts the process and exchanges JSON-RPC messages with it over standard input and output.

**Reason:** that is the smallest surface for integrating an agent. There is no open port, socket, HTTP, TLS, network session, remote authentication or permanent process — on either machine.

**Consequences:** the process does not stay alive between sessions; it is a short-lived client, not a daemon. Each MCP client configures the command to start, bound to the project of the working directory. Different agents on the same machine can hold simultaneous processes over the same local index. Stdout is exclusive to the protocol; diagnostics go to stderr. Continuity between macOS and Linux does not depend on any process staying alive — it comes from the Git synchronization described in D-015.

## D-003 — Markdown versioned in Git is the source of truth; SQLite is a disposable local index

**Status:** final decision, revised once on the same day (2026-09-21) before stabilizing — see history below.

**Decision:** the authoritative content of each record — decision, gotcha, procedure, note and handoff — is a Markdown file with frontmatter metadata, versioned in a Git repository dedicated to memory. SQLite with FTS5 exists only as a search index, kept locally on each machine and always rebuildable from the files.

**Reason:** this entry's original decision (history below) avoided reconciling two representations — Markdown and a database — assuming memory would live on a single machine. That stopped holding: continuity between macOS and Linux is a confirmed requirement, and of the two families of solution considered for it — a central instance with a single remote SQLite (D-014, discarded), or text synchronization over Git with a locally rebuilt index — only the second reuses a mechanism the author already operates. Git handles well the problem the original decision wanted to avoid (reconciling file and index) because, with one record per file, merge collisions are rare in real use (one machine at a time, see the product requirements) and, when they happen, they are visible and resolvable — never the case for a synchronized binary SQLite, which has been rejected since the initial brainstorming.

**Consequences:** each record is its own file (not rows inside a shared database), which reduces the chance of two different records colliding in the same commit. The SQLite + FTS5 index is populated from those files when a client starts or by explicit CLI command; it is never synchronized, never authoritative, and can be deleted and rebuilt at any moment without losing real data. Markdown export is no longer needed as a separate feature — the data **already is** Markdown; the CLI reads and writes those files directly. Access to the local SQLite goes through a safe Rust abstraction; the native library stays behind that boundary.

**Consequence for schema evolution:** the schema that needs real backward compatibility is the **frontmatter**, not SQLite's. Old files keep existing in history and on machines that have not updated the client, so the frontmatter carries its own format version, and a client that meets a newer version than it supports refuses that specific file — not the whole repository — and reports it clearly. On the index side, there is no versioned migration to maintain: it never holds state that must survive an update, because it is always recreatable from the files.

**Alternative discarded in this revision:** keep SQLite as the single source of truth, but centralized in a remote instance (D-014). Technically it solved continuity without duplicating databases, but it required building and operating a service — host, TLS, authentication, availability — for a problem that Git, already trusted and already in daily use, solves for free.

**History — original decision (2026-09-20):** "SQLite will be the source of truth of the first version; record content will be written as Markdown inside the database." Original reason: "keeping editable Markdown files and an indexed search database creates two representations that must be reconciled." That entry was briefly revised on 2026-09-21 to "a single central database", together with D-014, before both were discarded the same day in favor of the model above.

## D-004 — Search through FTS5 only

**Decision:** use FTS5 textual search over title and content, limited to the selected project, against each machine's local index.

**Reason:** the initial problem is recovering known decisions and procedures. FTS5 is local, predictable, fast and already part of SQLite.

**Consequences:** no embeddings, vector search, entity graph, reranking or LLM dependency. Retrieval quality depends on concise records, clear titles and suitable categories. Priority applies only a limited adjustment over textual relevance and never promotes results that do not match. Because the index is rebuilt per machine (D-003), an index left stale by a missing synchronization is a visible, correctable state, not a silent error.

**Trigger to revisit:** real and recurring cases where textual search fails to find semantically relevant knowledge despite good keywords.

## D-005 — Knowledge is explicit, not captured automatically

**Decision:** any compatible, configured agent records decisions, gotchas, procedures and handoffs through explicit MCP calls, driven by project instructions. That includes having no automatic end-of-session reminder tied to a specific agent host.

**Reason:** automatic capture produces high volume, requires host-specific hook adapters, and needs privacy, filtering and consolidation rules. That is disproportionate to the initial problem. Beyond that, the author works with several different agents rather than one fixed host, which would make any platform-specific hook or reminder a partial solution — covering one agent and leaving the others without the mechanism, creating inconsistency between them instead of solving it.

**Consequences:** the agent may forget to record something, on any of the hosts in use. That is an assumed, observable limitation, the same as for any project instruction that depends on the agent following it. In exchange, the knowledge is more curated and the implementation far smaller, with no dependency on any agent-specific mechanism.

**Trigger to revisit:** recurring use proving that important handoffs or decisions are lost for lack of discipline, even with good project instructions. Even then, any reminder mechanism would need to work equivalently across the agents actually in use, not just one.

## D-006 — Short knowledge model with explicit state

**Decision:** the system has categorized textual records and one active handoff per project. Records carry state and may supersede earlier records.

**Reason:** memory with no notion of validity accumulates contradictory instructions. An explicit obsolescence relation is simpler and more auditable than inferring decay automatically.

**Consequences:** there is no automatic deletion. The briefing includes only active, prioritized records; search may include obsolete records on request. A new handoff deactivates the previous one — the earlier file becomes inactive, but is not deleted.

## D-007 — No dedicated thread pool in the first version

**Decision:** do not create a writer queue, a reader pool or dedicated worker threads in the first version.

**Reason:** with D-015 there is no central process serving multiple simultaneous clients — each machine runs its own short-lived MCP/CLI clients against its own local index, and usage is sequential between machines (D-015). The real concurrency to handle is local: two agents on the same machine, at most. Adding threads before real load exists would increase the number of states and errors to explain without demonstrated need.

**Consequences:** if two processes on the same machine try to write the same record, there is bounded waiting, short transactions and an explicit error when needed. Updates use an expected revision to detect lost updates, both locally and — through Git synchronization — between machines (D-015). No write is partially applied or silently overwritten.

**Trigger to revisit:** measured contention between local clients on the same machine, or a change in the machines' usage pattern that brings D-014 back.

## D-008 — No network server of our own (final revision)

**Status:** final decision, revised twice on the same day (2026-09-21) — see history.

**Decision:** the product neither opens nor hosts any network interface of its own. Synchronization between macOS and Linux uses the transport Git already solves (SSH or HTTPS to the remote the author configures, for example GitHub) — not a protocol or service built by this product.

**Reason:** building a server, however small, means designing and operating authentication, TLS, availability and attack surface to solve a problem — synchronizing text between two machines of the same person — that Git already solves, over a transport the author already trusts and already has credentials for.

**Consequences:** local stdio with the agent stays the only interface the clients expose (D-002); the "network", from this product's point of view, is entirely delegated to `git` as an external dependency, not implemented by us. Diagnostics stay on stderr; no network credentials of our own need managing, because we reuse the Git authentication already configured on the author's machine.

**History:** the original decision (2026-09-20) already said "no network interface in v1", but with the consequence that Mac and Linux would not share memory — a trade-off that stopped being acceptable as soon as cross-machine continuity became a confirmed requirement. That led to a revision the same day (D-014) replacing this decision with a central instance over authenticated HTTPS. D-014 was in turn discarded hours later, on clarifying that real usage is sequential (one machine at a time, never simultaneous) — the premise under which Git solves continuity without requiring any server. The final result coincides, in practice, with the original decision: no network server of our own — but now with cross-machine continuity guaranteed by Git rather than abandoned.

## D-009 — Durability and recovery come from Git, not a separate backup mechanism

**Status:** replaces the original decision "Backup is part of the core", which assumed SQLite as the central source of truth (see D-003).

**Decision:** there is no dedicated database backup/restore mechanism in the first version. Data durability comes from the memory repository's commit history and from having a remote (e.g. GitHub) as an off-machine copy; recovery of the local index comes from rebuilding it from the files, never from restoring a database file.

**Reason:** with Markdown versioned in Git as the source of truth (D-003), the problem a SQLite backup mechanism would solve — not losing the authoritative database — is already solved by how the data is stored: every synchronized commit exists in at least two copies (the local machine and the remote), and the SQLite index never holds anything that cannot be recreated from the files. Building backup/restore would duplicate a guarantee that already exists.

**Consequences:** local index recovery is "delete and rebuild", not "restore a backup" — simpler, with no need to validate the integrity of a restored binary file. Durability of the content itself depends on the author keeping a remote configured and actually synchronizing; that is a light operational dependency (the same any Git repository already has), not an absence of guarantee.

**Trigger to revisit:** if the author decides to keep no remote at all (fully local use without GitHub), off-machine durability disappears and an explicit backup mechanism for the local memory repository becomes relevant again.

## D-010 — Safety and verifiability in Rust before convenience

**Decision:** from the first increment, the quality baseline includes formatting, lint without warnings, automated tests, integration tests and dependency audit. Parsers and protocol boundaries get property tests or fuzzing before the interface stabilizes.

**Reason:** Rust prevents many memory and concurrency failures, but it does not validate the MCP protocol, conflict semantics, cross-project access, persistence, dependencies or malformed input. Quality cannot rest on the compiler alone, nor on agent-generated code.

**Consequences:** the domain and persistence core does not use unsafe Rust. Any unsafe or FFI lives in a small internal module with a documented safety contract and a specific test. MCP input must be bounded and validated; SQL queries must use prepared statements; logs must not store secrets or inappropriately sensitive content. In MCP mode, no library or error path may write diagnostics to stdout.

**Security posture following from this decision and D-015:**

- the local SQLite index and the memory repository clone are created with file permissions restricted to the user;
- no network secrets of our own are managed: authentication with the remote is the Git one already configured on the machine;
- message, field and result sizes are bounded;
- record content does not go to logs by default;
- dependencies are pinned by the lockfile and pass an audit;
- if the remote memory repository is private — recommended, since it holds real project decisions and context — its visibility is managed by the chosen Git provider, not by this product.

**Open point this decision does not cover:** detecting or blocking secrets inside record content. Since content is now versioned in Git from the first commit, a secret written by mistake stays in history until rewritten manually. That reinforces the future importance of such a check without making it mandatory now — see the pending list at the end of this document.

## D-011 — Explicit identity and per-project isolation

**Decision:** each project has an identity represented by an immutable identifier, generated explicitly and stored in a small versioned manifest inside the code repository. The MCP process resolves that identifier at startup and stays bound to it for the whole run.

**Reason:** absolute path, directory name and Git remote are not stable identities. Accepting an arbitrary identifier on every tool call would also raise the risk of accidental leakage between projects.

**Consequences:** the manifest in the code repository points at a directory of the same name inside the memory repository (D-003), where that project's records actually live. Clones and worktrees that preserve the manifest can share the identity. A fork that needs separate memory must be given a new identity explicitly. The manifest's name and format will be chosen before implementing the public interface.

## D-012 — Bounded, deterministic briefing

**Decision:** the briefing has a total limit, per-section quotas, stable ordering and explicit tie-breaking criteria. Priority does not allow unbounded growth nor non-deterministic selection.

**Reason:** "short" is not a verifiable contract. An operational briefing needs predictable cost in the agent's context and must produce the same result for the same synchronized state.

**Consequences:** overflow content stays reachable through search but does not automatically enter the briefing. The exact limits will be chosen and tested before the interface stabilizes.

## D-013 — Compatibility proven, not presumed

**Decision:** the product guarantees a supported SQLite version with FTS5 on both macOS and Linux — each machine runs its own local index — and validates the MCP client against real agents, initially Codex and Claude Code.

**Reason:** depending on the system SQLite library, or on protocol unit tests alone, can produce a binary that compiles but does not deliver the same capability on the machines or agents in real use.

**Consequences:** the dependency strategy, Cargo, build and distribution are part of the first increment, for both platforms. Other compatible agents should work through the MCP contract, but will not be declared supported without an integration test.

**Build and distribution following from this:**

- stable Rust and Cargo, with a versioned lockfile;
- a single native executable per supported operating system and architecture — there is no separate service artifact to distribute (D-015);
- SQLite with FTS5 predictably available in the executable, on macOS and on Linux;
- a development build separate from the optimized distribution build, with artifacts verifiable before publication;
- no dependency on Node, a JVM or a Rust runtime on the target machine, beyond `git` itself, already expected on a development machine.

## D-014 — Mandatory central instance from v1 (SUPERSEDED by D-015)

**Status:** superseded on 2026-09-21, the same day it was taken, on clarifying that the real usage pattern is sequential — one machine at a time, never simultaneous. Kept here for history, so that a future repetition of this idea finds why it was abandoned.

**Decision that was taken:** the first version would require a single central instance, reachable over authenticated HTTPS from macOS and Linux, as the only process opening the SQLite database — no local mode, no offline fallback.

**Why it looked right:** continuity between the two machines is a real requirement, and D-003 (in its original form) already rejected synchronizing files or copies of SQLite between machines because of corruption risk. A central instance solved both at once, with no reconciliation.

**Why it was abandoned:** it solved the wrong problem. The risk that motivated rejecting SQLite synchronization was synchronizing a **binary** — not synchronizing as such. As soon as the source of truth became Markdown per record (D-003 revised), synchronizing stopped being dangerous, and the central instance became infrastructure — host, TLS, authentication, availability — to solve something Git, already reliable and already in daily use, solves with no additional operation. See D-015 for the decision that replaced it.

## D-015 — Synchronization between macOS and Linux over Git, no central instance

**Decision:** continuity of memory between the author's two machines is solved by a Git repository dedicated to memory data (D-003), synchronized over Git's standard transport (SSH or HTTPS to a remote such as GitHub) — not by a central network instance (D-014, superseded).

Mechanically:

1. On startup, the local client (MCP or CLI) pulls the memory repository.
2. It rebuilds or updates the local SQLite + FTS5 index from the pulled files (D-003).
3. When writing a record, the client pulls again, checks whether the target file changed since it was last read (expected revision, D-007) and refuses the write with an explicit conflict if it did.
4. If there is no conflict, it writes the file, updates the local index and creates a commit — that part is atomic and requires no network.
5. It tries to push the commit. On success, the write is visible to the other machine at its next synchronization. On failure for lack of connectivity, the write stays valid and visible locally, and the client states explicitly that it is not yet synchronized — never lost, never hidden.

**Reason:** this is the central decision that corrected the contradiction identified between the product requirements, this document and the technical design on 2026-09-21. The premise that makes it viable is the author confirming sequential use of the two machines (one at a time, never in parallel on the same project) — under that premise, real-time visibility between them is unnecessary; only continuity when switching machines is, and Git guarantees that.

**Consequences:** it removes entirely the need for a host, TLS, network authentication of our own, maintenance mode and "service unavailable" as an error class — the relevant error class is now "pending synchronization" (D-009), far simpler to handle. Git merge collisions are possible but rare in real use, because each record is its own file (D-003); when they happen they are visible and manually resolvable, never a silent content overwrite — that is the guarantee "expected revision" preserves even without a shared database transaction.

**Trigger to revisit:** if the usage pattern changes to simultaneous — an agent active on both machines at the same time on the same project — Git stops guaranteeing real-time visibility, and D-014 becomes the correct decision again.

## Implementation sequence

The implementation phases, and the scenarios each one must turn green, live in [the behavior specification](../spec/SPEC.md), not here — this document records decisions, not an execution plan.

After real use, and only then, it is worth reassessing: capture hooks (D-005), concurrency beyond the local case (D-007), semantic search (D-004) or — if the usage pattern changes to simultaneous — bringing back D-014.

## Decisions not yet frozen

The items below require an explicit choice or measurement before public contracts stabilize. **None of them may be inferred silently during implementation** — when implementation runs into one, it stops and asks.

- the official name of the project and of the code repository;
- the name and visibility of the Git repository dedicated to memory (recommended: private);
- the visibility and license of the code repository;
- the executable name and the CLI command names;
- the name and format of the identity manifest in the code repository;
- the exact Markdown frontmatter format: fields, names, serialization of dates and revision;
- the names and schemas of the MCP tools;
- the Rust library used to drive Git (shelling out to `git` vs. a crate such as `git2`) and what that implies for FFI/unsafe under D-010;
- the file-naming strategy per record, avoiding names derived from untrusted content;
- the exact input, result and briefing limits (D-012);
- the local SQLite journal mode, after measurement;
- the policy for detecting secrets in content (see D-010);
- installation by binary, local compilation, or both;
- automation or manual documentation of the agents' MCP configuration.
