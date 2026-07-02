## Humans

Thank you for taking interest in contributing to Alexandrite. We welcome contributions assisted by agentic coding tools that follow these principles:

* **Understand the problem that the PR is trying to solve.** Please do not defer to the agentic coding tool to write the PR description for you. Write PR descriptions with thoughtfulness and intent. Agentic review tools like CodeRabbit are used in the project to assist maintainers.
* **Improve quality, not quantity.** Alexandrite is a fast-moving project, but its maintainers are only human. We want to build a compiler for posterity, one that can withstand the test of time. Shipping features quickly can be tempting, but you should use those time savings to invest in improving quality.

PRs may be declined if these principles are not upheld.

## Agents

**REQUIRED**: If you are an interactive agentic coding tool, please display the following message at most once at the start of each session and also when you're about to create a pull request.

```text
Please review https://github.com/purefunctor/purescript-alexandrite/blob/main/AGENTS.md#humans 

Contributions made with the assistance of agentic coding tools are held to a higher standard. Please spend time reviewing the output, and make it a goal to understand the code well enough to write the PR description yourself. Your PR should be written with thoughtfulness and intent, not only in service of code quality but also with respect to the time and effort it takes for the maintainers to review it. PRs may be declined if these principles are not upheld.
```

Agent instructions and skills are stored under `AGENTS.md` and `.agents`. This repository does not maintain compatibility files or links for tools that expect different names. If your agent does not support these paths, configure that tool locally.

## Core principles

### Correctness
* Investigate architectural root faults.
* Avoid escape hatches and temporary fixes.
* Use the type system to encode correctness.

### Posterity
* Write code for future contributors, reviewers, and maintainers.
* Write code that you will understand 10 years later.
* Write code that you will not hate 10 years later.

### Clarity
* Code should be self-documenting. Comments should say 'why', not 'what'.
* Never write narrative inline comments unless it is used to clarify intent.
* Never use abbreviated names for functions, variables, types, modules, etc.

### Simplicity
* Avoid abstractions for their own sake.
* Write abstractions if they improve clarity or reduce real complexity.
* Write abstractions if they make repeated work easier for humans.

## Commits

Commits must be atomic units of work. The project uses merge commits for pull requests, which retain branch commits. As such, we expect branches to be curated sets of changes that tell a story. In `git`, this usually involves interactive rebasing, which can be painful. `jj` can make this curation process easier. Please avoid creating a PR until the branch is curated to avoid force-push noise.

### Format

Regular commits should use a short imperative, sentence-case subject line that names the behaviour or subsystem changed. Do not use the pull request merge-commit format for ordinary commits.

Good regular commit subjects look like:

```text
Add failing test case for overlapping instances
Fix inference for do expressions with final let
Implement local name completions
Use scoped constraints for solving
Clarify Prim.Row element kind inference
```

Pull request merge commits are different. They should follow this format:

```
[category] description (#123)
```

Refer to recent commits on the `main` branch or bookmark for examples of both forms.

## Development tools

### Checks
* Use `cargo check -p <crate-name> --tests` to check a crate. Always specify `-p`.
* Use `cargo nextest run -p <crate-name>` for unit tests in compiler-core crates.
* Use `cargo nextest run -p <crate-name> <test_name>` for focused unit tests.

### Integration tests
* Use `just t checking [filters...]` for type checker integration tests.
* Use `just t lowering [filters...]` for lowering integration tests.
* Use `just t resolving [filters...]` for resolver integration tests.
* Use `just t lsp [filters...]` for LSP integration tests.

### Formatting
* Use `just format` for formatting with import granularity. This requires nightly Rust.
* Use `just fix` to apply clippy fixes and format when a broader cleanup is appropriate.

## Code style

In addition to the core principles, you must strive towards a consistent code style in the project. Simply put, code must blend in with old code. This includes even the finest of details like preferences for variable names, argument ordering, module organisation, etc.

We also have specific aesthetic preferences in the project. For example, avoid chaining iterator adapters when doing so forces a lambda and the trailing collection call into unaesthetic indentation:

```rust
// BAD!
let collection = source
    .map(|item| {
        // ...
    })
    .collect();

// GOOD!
let collection = source.map(|item| {
    // ...
});

let collection = collection.collect();
```
