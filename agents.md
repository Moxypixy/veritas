# AGENTS.md
# Project Engineering Instructions

This project builds applications for Kaspa using the experimental Argent
language together with Rust.

Argent is experimental and evolving rapidly. Never assume Argent syntax,
compiler behaviour, runtime behaviour, generated artifacts, APIs, conventions,
or Kaspa integration details from general knowledge.

The repository and the current Argent source code are the source of truth.

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
understanding how the existing Argent and Rust code works.

For bugs, use systematic debugging rather than speculative fixes.

Never bypass tests, compiler errors, type errors, lint rules, or security checks
simply to make a task appear complete.

---

# 3. Sources of Truth

For this project, use sources in this order:

1. The current local project source code.
2. The current local Argent repository.
3. Current official Argent documentation/examples.
4. Current official Kaspa repositories/documentation.
5. Current dependency documentation via Context7.
6. General model knowledge only when the above do not answer the question.

Argent is experimental.

NEVER invent Argent syntax, compiler features, APIs, runtime behaviour,
transaction semantics, covenant behaviour, or interoperability.

If uncertain:

- inspect the current Argent repository;
- inspect existing `.ag` examples;
- inspect compiler tests;
- inspect generated artifacts;
- consult current upstream documentation;
- state uncertainty explicitly if behaviour cannot be verified.

Use Context7 for current documentation for third-party libraries and frameworks.

Do not use outdated remembered APIs when current documentation is available.

---

# 4. Existing Argent Template Architecture

Preserve the repository's basic separation unless there is a strong reason to
change it.

Current responsibilities:

- `ag/`
  Argent applications and covenant/domain logic.
- `src/bin/`
  Rust executables responsible for transaction construction and application
  execution/integration.
- `src/lib.rs`
  Shared Rust utilities and deterministic local fixtures.
- `build/`
  Generated Argent build artifacts.

Do not move domain rules into frontend code.

Do not duplicate Argent validation rules in a frontend and treat the frontend
copy as authoritative.

The Argent program is authoritative for rules that must be enforced by the
Kaspa-side application.

Rust should handle orchestration, integration, transaction construction, data
conversion, and supporting infrastructure.

Frontend applications should handle presentation and user interaction.

---

# 5. Experimental Argent Safety Rules

Argent must be treated as experimental software.

Before changing Argent code:

1. Inspect similar working `.ag` files.
2. Verify syntax against the current compiler.
3. Verify assumptions against the current Argent checkout.
4. Build the program.
5. Inspect the generated artifact when relevant.
6. Test expected and invalid paths.

Do not infer features from Rust, Solidity, Move, Bitcoin Script, or another
smart-contract language.

Do not assume Solidity/EVM concepts apply to Argent.

Do not assume account-model semantics.

Do not assume transaction or UTXO behaviour that has not been verified.

Do not create compatibility shims around misunderstood compiler behaviour.

When the compiler rejects something, understand why before working around it.

---

# 6. Argent Code

Argent code should:

- model domain rules explicitly;
- keep actors, state and allowed transitions understandable;
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

Keep Argent-generated artifact handling isolated from unrelated application
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

The core Argent/Rust application must not depend on one particular frontend.

Frontend implementations may include:

- React web applications;
- React Native applications;
- Python clients;
- CLI tools;
- future clients.

Keep the domain/integration boundary independent of UI technology.

Prefer an explicit interface between frontend clients and the Rust/Argent
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

Do not duplicate authoritative Argent business rules in Python.

Python clients must treat responses and external data as untrusted.

---

# 14. API / Boundary Design

If a frontend communicates with a Rust service, design the boundary independently
of the frontend framework.

Prefer explicit typed request/response contracts.

Validate all external input at the Rust boundary.

Frontend validation improves UX but is NEVER a security boundary.

Avoid leaking internal Argent artifact representation directly into UI code.

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

The starter template currently demonstrates Argent using its local runtime.

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

Generated Argent artifacts are outputs, not hand-maintained source.

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

For security-critical Argent logic, rejection tests are as important as success
tests.

Do not remove failing tests merely to make CI pass.

Do not weaken assertions without understanding the reason.

---

# 19. Commands and Verification

Use the repository's existing commands where applicable.

Argent:

    ./argentc build ag/<application>.ag

Inspect generated Argent output when relevant:

    ./argentc inspect build/argent

Rust:

    cargo fmt --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test --all-features

Run a binary when appropriate:

    cargo run --bin <name>

Run repository checks:

    ./check

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

For Argent code, additionally review whether a maliciously constructed
transaction could satisfy the covenant while violating intended application
behaviour.

---

# 22. Comments and Documentation

Comment WHY, not obvious syntax.

Document:

- unusual Argent behaviour;
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
2. Review Argent assumptions against the current toolchain.
3. Build all changed Argent programs.
4. Run Rust formatting.
5. Run Clippy with warnings denied.
6. Run Rust tests.
7. Run `./check`.
8. Run frontend lint/typecheck/tests when applicable.
9. Review the complete diff.
10. Check security-sensitive changes separately.
11. Remove debugging output and dead code.
12. Confirm documentation still matches behaviour.
13. Report what was actually verified.

If something could not be verified, state that explicitly.

Never report success based on assumption.
