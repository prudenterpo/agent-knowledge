# Engineering Constitution — Agent Knowledge

## Purpose and authority

This constitution defines how Rust code in this repository is designed, written, tested and reviewed. It preserves the product constraints in the behavior specification and the technical decisions; it does not replace either document.

When a rule conflicts with a documented product decision, the product decision wins. A deliberate exception to this constitution must be narrow, explained in the pull request, and covered by a test when behavior or safety is affected.

The sources behind these rules are the Rust API Guidelines, The Rust Programming Language, Rustfmt and Clippy. They are adopted selectively for a small local system, not as a mandate to maximize lint count or abstraction.

## Design rules

- Keep MCP and CLI adapters thin. They validate transport input, call the application core and map results to their own protocols. They do not contain domain rules.
- Keep the domain and application core independent from JSON-RPC, terminal parsing, Git processes, SQLite and filesystem paths. Put those details behind small interfaces owned by the outer layers.
- Model valid states in types. Use enums for mutually exclusive states and small newtypes for identifiers where confusing one value for another would violate project isolation or revision checks.
- Prefer direct data flow and a small module tree over frameworks, service locators, global mutable state or traits introduced before there is a second real implementation.
- Prefer borrowing to cloning. A clone at a process, persistence or asynchronous boundary is acceptable when it makes ownership explicit; it must not be used to silence a borrow-checker error without understanding the lifetime.
- Do not add concurrency, asynchronous runtime, shared mutable state, caches or background workers without a measured requirement. The first version handles bounded local contention with short operations and explicit conflicts.
- Do not use unsafe Rust in the domain or application core. An unavoidable unsafe or FFI boundary lives in one small module, has a written safety invariant, and has a focused test.

## Idiomatic Rust

- Use stable Rust, rustfmt and conventional Rust names: modules, functions and variables use snake_case; types and traits use PascalCase; constants use SCREAMING_SNAKE_CASE.
- Make an operation a method when one input is clearly its receiver. Use standard conversion traits and conventional conversion names instead of ad-hoc alternatives.
- Return Result for expected operational failures and use a typed error model that preserves the error categories defined by the technical design. Do not use panic, unwrap or expect for malformed input, I/O, persistence, Git, SQLite or protocol failures.
- Use Option only when absence is a normal state. When more than two states matter, define an enum with meaningful variants rather than nesting Option values.
- Keep functions focused on one operation. Extract a helper when it creates a reusable domain concept or removes a meaningful branch of complexity, not merely to shorten a function.
- Prefer standard-library types and a mature narrow dependency over handwritten parsers, shell protocols or unsafe native bindings. Every dependency must solve a concrete product problem and be justified in its pull request.
- Do not expose a type, field or function as public merely to make tests convenient. Test through the smallest meaningful interface.

## Comments and Rustdoc

Code must make the normal path understandable without commentary. A comment is allowed only when it explains one of these: a non-obvious invariant, a business or compatibility reason, a safety condition, an external protocol constraint, or why a rejected-looking alternative is intentionally retained.

- Do not write comments that paraphrase the next line, narrate control flow, preserve temporary thoughts, or decorate obvious code.
- Prefer a precise identifier, type, function extraction or test name when that removes the need for a comment.
- Keep an allowed comment adjacent to the code it constrains and as short as the invariant permits. Avoid block comments.
- Remove a comment when the code, constraint or workaround it describes disappears.
- Rustdoc belongs on package-boundary public APIs and on unsafe interfaces, not on every internal item. It explains what the caller needs to know; when applicable it has Errors, Panics or Safety sections. Examples are included when they clarify intended use rather than restate syntax.

## Persistence, errors and resources

- Treat Markdown files as authoritative and SQLite as rebuildable derived state. Never make a repair depend on SQLite being correct.
- Validate all input at the boundary, bound allocation and result sizes, and use prepared SQL statements. Never log record bodies, credentials or tokens by default.
- Make durable file changes through a temporary sibling and atomic rename. Git commits contain complete authoritative file changes; SQLite files never enter Git.
- Preserve original errors with useful context at a boundary, but do not leak library internals or sensitive data through MCP or CLI messages.
- Hold database transactions, locks and filesystem handles only for the shortest operation that requires them. Destruction must not be the only path that reports an operational failure.

## Tests and quality gates

- A test proves observable behavior from the specification, not an implementation detail. Unit tests cover deterministic domain rules; integration tests cover files, SQLite, Git and adapters at their real boundaries.
- Use temporary directories and explicit fixtures. Do not depend on developer machine paths, current time without a controlled clock, network availability, execution order or sleep-based synchronization.
- Add a regression test with every bug fix. Add property tests or fuzzing for parsers and protocol inputs before their contracts are declared stable.
- Before a phase is complete, formatting, linting without warnings, tests, documentation build and the configured dependency audit must pass on macOS and Linux.
- Clippy starts with its standard warnings treated as errors. Pedantic, nursery and restriction lints are evaluated individually; they are not enabled as a blanket policy because signal matters more than lint volume.

## Review checklist

- Does the code keep the domain independent from adapters and infrastructure?
- Are ownership, mutation and failure paths explicit without unnecessary cloning or shared state?
- Would a clear name or small extraction remove each comment?
- Are public contracts documented and are error, panic and safety conditions stated where relevant?
- Is each dependency, unsafe boundary and exception justified by a real requirement?
- Do the tests demonstrate the specified behavior and the required quality gates pass?
