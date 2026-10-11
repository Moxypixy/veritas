# AGENTS.md
# Project Engineering Instructions

This project builds Veritas, verifiable price records on Kaspa L1, using
Silverscript covenants together with Rust and a TypeScript web client.

Silverscript (kaspanet/silverscript, pinned to v1.0.0) is the covenant
language. It is young and evolving. Never assume Silverscript syntax, compiler
behaviour, ABI artifacts, covenant declaration semantics, or Kaspa (Toccata,
transaction v1) details from general knowledge.

The repository, the pinned Silverscript source and the pinned rusty-kaspa
revision are the source of truth. Argent is no longer used; treat it only as
a design reference for multi-contract systems.

---

# 1. Core Principles

Prioritise:

1. Correctness
2. Security
3. Simplicity
4. Explicit behaviour
5. Maintainability
6. Testability
7. Accessibility
8. Performance

Prefer boring, understandable code over clever abstractions.

Follow separation of concerns.

Follow DRY when duplication represents the same domain concept, but do not
create abstractions merely to eliminate a few repeated lines.

Do not introduce speculative abstractions for future requirements.

Keep changes small and reviewable.

---

# 2. Development Workflow

Use the Superpowers workflow for development tasks.

For substantial features:

1. Understand the requirement.
2. Inspect the existing implementation.
3. Brainstorm the design.
4. Produce an implementation plan.
5. Identify security and trust boundaries.
6. Write tests first where practical.
7. Implement the smallest correct solution.
8. Run all relevant checks.
9. Review the final diff.
10. Perform a specialist security/correctness review.
11. Verify the result before declaring completion.

Do not start implementing significant architectural changes without first
understanding how the existing Silverscript and Rust code works.

For bugs, use systematic debugging rather than speculative fixes.

Never bypass tests, compiler errors, type errors, lint rules, or security checks
simply to make a task appear complete.

---

# 3. Sources of Truth

For this project, use sources in this order:

1. The current local project source code.
2. The pinned Silverscript source (docs/TUTORIAL.md, docs/DECL.md, tests).
3. The Toccata developer guide (docs.kaspa.org/toccata) and current KCCs.
4. Current official Kaspa repositories/documentation.
5. Current dependency documentation via Context7.
6. General model knowledge only when the above do not answer the question.

Silverscript and Toccata are new.

NEVER invent Silverscript syntax, compiler features, APIs, runtime behaviour,
transaction semantics, covenant behaviour, or interoperability.

If uncertain:

- inspect the pinned Silverscript repository;
- inspect existing `.sil` examples and its compiler tests;
- compile with `silverc` and inspect the ABI artifact;
- execute the script in rusty-kaspa's engine (see `crates/veritas-anchor/tests`);
- consult current upstream documentation;
- state uncertainty explicitly if behaviour cannot be verified.

Use Context7 for current documentation for third-party libraries and frameworks.

Do not use outdated remembered APIs when current documentation is available.

---

# 4. Repository Architecture

Preserve the repository's basic separation unless there is a strong reason to
change it.

Current responsibilities:

- `covenants/`
  Silverscript covenants. `anchor.sil` holds the append-only accumulator.
- `src/` (crate `veritas`)
  Pure Rust core: price records, units, Merkle batches, accumulator,
  receipts and verification. No Kaspa or Silverscript dependencies.
- `src/bin/veritas.rs`
  Demo CLI: keygen, attest, batch, verify, unit-price.
- `crates/veritas-anchor/`
  Compiles `anchor.sil` and builds its sigscripts; tests run the covenant in
  rusty-kaspa's script engine.
- `crates/veritas-api/`, `crates/veritas-inflation/`
  Legacy Cost-of-Living Pulse API and official-statistics worker (to move to
  The Gap).
- `web/`
  React client. `src/veritas/verify.ts` mirrors the Rust verifier;
  `src/wallet/kaspire.ts` is the only wallet integration (Kaspire,
  `window.kaspire`, KIP-5 `signMessage`).
- `covenants/anchor.abi.json`, `covenants/anchor.template-hash.txt`
  Published KCC-1 Program ABI and template hash; regenerate with
  `cargo run -p veritas-anchor --bin anchor-abi` after any covenant change.

Conventions to keep: KCC-1 (BLAKE3 Hash, state encoding via Silverscript),
KCC-2 canonical scheme bytes for authorities, KCC-20 witness layouts where an
authority is checked, KIP-5 for message signatures. Veritas issues no token.
- `examples/demo/`
  Committed demo records; Rust and web tests both verify them.

Do not move domain rules into frontend code.

The browser verifier is a convenience mirror of `src/verify.rs`; both are
tested against `examples/demo`. If they disagree, Rust and the covenant win.

The covenant is authoritative for rules that must be enforced on Kaspa;
`veritas::accumulator::next_acc` must stay in step with `anchor.sil`.

Rust should handle orchestration, integration, transaction construction, data
conversion, and supporting infrastructure.

Frontend applications should handle presentation and user interaction.

---

# 5. Covenant Safety Rules

Silverscript and Toccata covenants must be treated as young software.

Before changing covenant code:

1. Inspect similar working `.sil` files in the Silverscript repository.
2. Verify syntax against the pinned compiler.
3. Verify assumptions against the pinned Silverscript and rusty-kaspa source.
4. Build the program.
5. Inspect the generated artifact when relevant.
6. Test expected and invalid paths.

Do not infer features from Rust, Solidity, Move, Bitcoin Script, or another
smart-contract language.

Do not assume Solidity/EVM concepts apply to Kaspa covenants. Model state as
UTXO transitions; covenant IDs gate admission but do not enforce app rules,
and the successor outputs must be validated, not just the inputs.

Do not assume account-model semantics.

Do not assume transaction or UTXO behaviour that has not been verified.

Do not create compatibility shims around misunderstood compiler behaviour.

When the compiler rejects something, understand why before working around it.

---

# 6. Covenant Code

Covenant code should:

- model domain rules explicitly;
- keep state fields and allowed transitions understandable;
- minimise hidden assumptions;
- make invalid state transitions impossible where practical;
- validate untrusted inputs;
- avoid unnecessary complexity;
- document non-obvious covenant or transaction assumptions;
- use meaningful domain names;
- favour explicit state transitions over clever shortcuts.

Security-sensitive logic must include tests covering rejection cases.

For every important state transition, consider:

- who may perform it;
- what state must already exist;
- what values may change;
- what must remain invariant;
- whether the transition can be replayed;
- whether inputs can be substituted;
- whether outputs are sufficiently constrained;
- whether value can accidentally escape;
- whether malformed transactions can satisfy the covenant.

Never treat a successful compile as proof of economic or security correctness.

---

# 7. Rust

Write idiomatic stable Rust unless the project explicitly requires otherwise.

Prefer:

- strong domain types;
- enums rather than magic strings or boolean combinations;
- explicit ownership;
- small focused modules;
- `Result` for recoverable failures;
- structured error types;
- deterministic functions where possible.

Avoid:

- unnecessary `clone()`;
- unnecessary allocations;
- unnecessary `Arc<Mutex<_>>`;
- global mutable state;
- hidden side effects;
- deeply nested control flow;
- panic-based error handling;
- premature generic abstractions.

Do not use `unwrap()` or `expect()` in production paths unless an invariant makes
failure genuinely impossible and the reason is documented.

At external boundaries:

- validate data;
- convert into domain types early;
- return structured errors.

Keep Silverscript artifact handling isolated from unrelated application
logic.

---

# 8. Rust Review Pass

After implementing Rust, perform a dedicated Rust review.

Check specifically for:

- unnecessary cloning;
- weak ownership modelling;
- avoidable allocations;
- excessive synchronization;
- incorrect lifetime assumptions;
- panic paths;
- inappropriate async usage;
- weak error modelling;
- non-idiomatic abstractions;
- unchecked conversions;
- integer overflow/underflow concerns;
- untrusted input handling.

Do not settle for "it compiles".

---

# 9. Frontend Independence

The core Rust/Silverscript application must not depend on one particular frontend.

Frontend implementations may include:

- React web applications;
- React Native applications;
- Python clients;
- CLI tools;
- future clients.

Keep the domain/integration boundary independent of UI technology.

Prefer an explicit interface between frontend clients and the Rust/Silverscript
application.

Do not embed React-specific, React-Native-specific, browser-specific, or
Python-specific assumptions inside core domain logic.

---

# 10. React + TypeScript

When a React frontend exists:

Use TypeScript with strict mode.

Do not use `any` unless there is a documented and unavoidable reason.

Prefer:

- semantic HTML;
- native browser behaviour;
- small focused components;
- composition;
- explicit props;
- derived state;
- domain logic outside presentation components.

Avoid:

- unnecessary `useEffect`;
- duplicated state;
- oversized components;
- stale closures;
- unnecessary memoisation;
- prop drilling when a clearer architecture exists;
- unnecessary dependencies.

Effects must clean up subscriptions, timers and event listeners.

Never trust data merely because it came from the frontend.

All security-sensitive validation belongs at the authoritative boundary.

---

# 11. Web Accessibility

Web interfaces must target WCAG 2.2 AA.

Prefer semantic HTML before ARIA.

All interactive functionality must:

- work with a keyboard;
- have a visible focus state;
- expose an accessible name;
- behave correctly with screen readers;
- not depend solely on colour;
- respect reduced-motion preferences where appropriate.

Forms must provide:

- associated labels;
- understandable validation;
- accessible error messages;
- clear success/error states.

Accessibility is part of correctness, not polish.

---

# 12. React Native

When React Native is selected:

Keep mobile UI code separate from shared domain/integration code.

Prefer platform-neutral TypeScript modules for reusable logic.

Do not assume browser APIs exist.

Do not place secrets or authoritative security logic in the application bundle.

Handle:

- app lifecycle;
- asynchronous storage;
- network interruptions;
- wallet handoff failures;
- deep-link validation;
- accessibility;
- loading/error/retry states.

Use current React Native documentation via Context7 before introducing APIs or
libraries.

---

# 13. Python Frontend / Client

Python may be used for:

- prototypes;
- desktop clients;
- data tooling;
- integration tools;
- testing;
- automation.

Use modern Python with type hints.

Prefer:

- small modules;
- explicit interfaces;
- dataclasses or typed models;
- structured exceptions;
- deterministic functions.

Use a type checker where practical.

Do not duplicate authoritative covenant rules in Python.

Python clients must treat responses and external data as untrusted.

---

# 14. API / Boundary Design

If a frontend communicates with a Rust service, design the boundary independently
of the frontend framework.

Prefer explicit typed request/response contracts.

Validate all external input at the Rust boundary.

Frontend validation improves UX but is NEVER a security boundary.

Avoid leaking internal Silverscript artifact representation directly into UI code.

Create stable application/domain representations where appropriate.

Errors crossing the boundary should be structured and safe to expose.

Never expose:

- private keys;
- seed phrases;
- signing secrets;
- internal credentials;
- sensitive implementation details unnecessarily.

---

# 15. Kaspa / Wallet Safety

Treat wallet interactions and transaction signing as high-risk boundaries.

Never:

- request seed phrases;
- log private keys;
- persist private keys unnecessarily;
- transmit signing secrets to application servers;
- silently sign transactions;
- hide transaction intent from users.

When wallet/network support is implemented, clearly separate:

1. transaction construction;
2. transaction presentation;
3. user approval;
4. signing;
5. submission;
6. confirmation.

Users should understand what they are authorising.

---

# 16. Current Template Limitation

Do not assume this template currently connects to the Kaspa network.

The v0 code compiles and tests the anchor covenant against rusty-kaspa's
script engine locally. Nothing yet submits the anchoring transaction.

Unless network integration has explicitly been added and verified, do not claim
that the application:

- connects to mainnet;
- connects to testnet;
- manages wallets;
- signs transactions;
- broadcasts transactions;
- confirms transactions on Kaspa.

When implementing network functionality, treat it as a separate integration
layer and verify it against current Kaspa APIs.

---

# 17. Generated Code and Artifacts

Generated Silverscript ABI artifacts are outputs, not hand-maintained source.

Do not manually edit generated files unless the toolchain explicitly requires
it.

Change the source and regenerate artifacts instead.

Keep generated output separate from authored application logic.

If generated artifacts unexpectedly change, review the diff before accepting it.

---

# 18. Testing

Tests should verify behaviour, not implementation details.

Cover:

- normal paths;
- invalid inputs;
- boundary values;
- state-transition failures;
- authorization failures;
- malformed data;
- transaction assumptions;
- serialization/deserialization boundaries;
- regressions for discovered bugs.

For security-critical covenant logic, rejection tests are as important as success
tests.

Do not remove failing tests merely to make CI pass.

Do not weaken assertions without understanding the reason.

---

# 19. Commands and Verification

Use the repository's existing commands where applicable.

Silverscript (from a checkout of kaspanet/silverscript at v1.0.0):

    cargo run -p silverscript-lang --bin silverc -- covenants/anchor.sil --constructor-args args.json -c

The covenant itself is compiled and executed by:

    cargo test -p veritas-anchor

Rust:

    cargo fmt --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test --all-features

Run a binary when appropriate:

    cargo run --bin <name>

Run repository checks:

    ./check.sh        (or check-win.ps1 on Windows)

If frontend tooling is present, also run its formatter, linter, type checker and
tests.

Do not claim that a task passes verification unless the relevant commands have
actually been executed successfully.

---

# 20. Dependencies

Do not add a dependency merely because it makes implementation easier.

Before adding a dependency:

1. explain why it is needed;
2. check whether existing dependencies solve the problem;
3. inspect maintenance and current documentation;
4. consider security implications;
5. use the smallest appropriate dependency.

For JavaScript packages, Rust crates and Python libraries, use current
documentation through Context7 when appropriate.

Pin or constrain versions appropriately.

Avoid abandoned or unnecessary packages.

---

# 21. Security Review

Before completing security-sensitive features, review:

- trust boundaries;
- authentication;
- authorization;
- transaction construction;
- signing;
- replay possibilities;
- state-transition invariants;
- input validation;
- integer arithmetic;
- serialization;
- secret handling;
- dependency risk;
- race conditions;
- error behaviour.

For covenant code, additionally review whether a maliciously constructed
transaction could satisfy the covenant while violating intended application
behaviour.

---

# 22. Comments and Documentation

Comment WHY, not obvious syntax.

Document:

- unusual Silverscript or Kaspa script behaviour;
- protocol assumptions;
- covenant invariants;
- security decisions;
- interoperability assumptions;
- temporary workarounds caused by experimental tooling.

Do not fill code with comments that merely repeat what the code says.

Experimental assumptions should be easy to find and remove later.

---

# 23. Architecture Changes

Before introducing a new architecture, framework, service, state-management
library or protocol:

1. explain the problem being solved;
2. describe the simplest alternative;
3. explain trade-offs;
4. verify compatibility with the current project;
5. obtain approval before making a large architectural migration.

Do not rewrite working code merely because another architecture is fashionable.

---

# 24. Definition of Done

A task is not complete because code was generated or because it compiles.

Before declaring completion:

1. Verify the requirement.
2. Review Silverscript and Kaspa assumptions against the pinned toolchain.
3. Run `cargo test -p veritas-anchor` after any covenant change.
4. Run Rust formatting.
5. Run Clippy with warnings denied.
6. Run Rust tests.
7. Run `./check.sh` or `check-win.ps1`.
8. Run frontend lint/typecheck/tests when applicable.
9. Review the complete diff.
10. Check security-sensitive changes separately.
11. Remove debugging output and dead code.
12. Confirm documentation still matches behaviour.
13. Report what was actually verified.

If something could not be verified, state that explicitly.

Never report success based on assumption.
