## Humans

Thank you for taking interest in contributing to Alexandrite. We welcome contributions assisted by agentic coding tools that follow these principles:

* **Understand the problem that the PR is trying to solve.** Please do not defer to the agentic coding tool to write the PR description for you. Write PR descriptions with thoughtfulness and intent. Agentic review tools like CodeRabbit are used in the project to assist maintainers.
* **Improve quality, not quantity.** Alexandrite is a fast-moving project, but its maintainers are only human. We want to build a compiler for posterity, one that can withstand the test of time. Shipping features quickly can be tempting, but you should use those time savings to invest in improving quality.

PRs may be declined if these principles are not upheld.

## Agents

The canonical specifications for agent instructions and skills are `AGENTS.md` and the `.agents` directory. If your agent does not support these specifications, you will have to configure it yourself.

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

### Commit format

Regular commits should use a short imperative, sentence-case subject line that names the behaviour or subsystem changed. Do not use the pull request merge-commit format for ordinary commits.

Good regular commit subjects look like:

```text
Add failing test case for overlapping instances
Fix inference for do expressions with final let
Implement local name completions
Use scoped constraints for solving
Clarify Prim.Row element kind inference
```

### Pull request title format

Pull request titles must follow this format:

```
[category] description
```

GitHub appends the pull request number when it creates the merge commit, producing `[category] description (#123)`. Do not include the pull request number in the title yourself.

Choose `category` for the primary subsystem or project area changed by the pull request. A category can be a crate name, such as `checking` or `analyzer`, or a broader project area, such as `lsp`. Use `agents` for agent configuration, `meta` for repository-wide maintenance, and `ci` for continuous integration changes. Use the narrowest established category that describes the change, consulting recent merge commits on `main` when necessary. If a pull request touches multiple areas, choose the category of its main intended outcome; do not list multiple categories.

Good pull request titles look like:

```text
[checking] Preserve type variable names in instance members
[lsp] Handle rename rejections
[analyzer] Collect diagnostics through analyzer hosts
[agents] Clarify pull request title categories
[meta] Update repository maintenance tooling
[ci] Test installers on supported platforms
```

Bad pull request titles look like:

```text
Preserve type variable names in instance members       # Missing category
[fix] Preserve type variable names in instance members # Describes the change type, not the subsystem
[checking/lsp] Improve rename errors                   # Lists multiple categories
[misc] Update inference                                # Uses a vague category despite a clear subsystem
```

## Development tools

### Checks
* Use `cargo check -p <crate-name> --tests` to check a crate. Always specify `-p`.
* Use `cargo nextest run -p <crate-name>` for unit tests in compiler-core crates.
* Use `cargo nextest run -p <crate-name> <test_name>` for focused unit tests.

### Snapshots
* Never edit `.snap` files by hand. Regenerate them through the test or snapshot-acceptance command that owns them.
* Use `just t <category> [filters...] --accept` to accept integration-test snapshots, or `cargo insta` for crates not covered by `just t`.
* Inspect every generated `.snap` diff and commit only changes that directly describe the intended behaviour.

### Integration tests
* Use `just t checking [filters...]` for type checker integration tests.
* Use `just t lowering [filters...]` for lowering integration tests.
* Use `just t resolving [filters...]` for resolver integration tests.
* Use `just t lsp [filters...]` for LSP integration tests.

Filtered fixture runs are useful while iterating, but they are not sufficient before pushing. Before pushing a change that affects integration tests, run `just t <category>` without fixture filters for every affected category and confirm that the entire category passes with no pending snapshots.

### Formatting
* Use `just format` for formatting with import granularity. This requires nightly Rust.
* Use `just fix` to apply clippy fixes and format when a broader cleanup is appropriate.

## Code style

In addition to the core principles, follow the project's existing conventions for variable names, argument ordering, module organisation, and formatting.

The following styles are required:

* Always bind an iterator expression to a local variable before collecting or folding it.
* Use the concrete type name instead of `Self` outside trait definitions and trait implementations.
* Keep expression complexity to a minimum by using intermediate bindings, but avoid writing A-normal form.

For example:

```rust
// Bind iterator expressions before consuming them.
let collection = source.map(|item| {
    // ...
});

let collection = collection.collect();

// Name concrete types in inherent implementations.
impl Span {
    pub fn new(start: u32, end: u32) -> Span {
        Span { start, end }
    }
}

// Name meaningful intermediate results while keeping simple expressions inline.
let absolute_path = fs::canonicalize(&source.path)?;
let uri = Url::from_file_path(&absolute_path)
    .map_err(|_| Error::FileUrl(absolute_path.clone()))?
    .to_string();
let file_id = files.insert(uri, content.clone());
engine.set_content(file_id, content);
```
