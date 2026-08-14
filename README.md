<h1 align="center">alexandrite</h1>
<p align="center">a language implementation for PureScript</p>

---

Alexandrite is a language implementation for PureScript, powered by an incremental, query-based build
system. Instead of a sequence of compiler phases, Alexandrite models compilation and semantic information
as incrementally computed queries. These queries are used extensively to implement code intelligence
features in the language server.

The build system is designed with interactive editing in mind. To support this, it tracks dependencies
between inputs and queries, caches query results, deduplicates in-progress work across threads, and
supports cooperative cancellation when inputs change. Crucially, many query results are designed to
be incrementally reusable. For example, the compiler uses stable identities in lieu of source ranges 
to enable minimal recomputation across trivial formatting changes.

The language server component implements core code intelligence features such as completion, jump to
definition, hover information, find references, workspace symbol search, and diagnostics.

## Editor features

Alexandrite provides code intelligence for PureScript projects through its VS Code extension.

| Completion | Automatic imports |
| :---: | :---: |
| ![Completing a PureScript expression](.github/assets/vscode-demos/completion.gif) | ![Automatically importing a completed PureScript name](.github/assets/vscode-demos/automatic-import.gif) |
| **Live diagnostics** | **Inferred types** |
| ![Updating diagnostics while editing PureScript](.github/assets/vscode-demos/live-diagnostics.gif) | ![Viewing an inferred PureScript type](.github/assets/vscode-demos/inferred-types.gif) |
| **Go to definition** | **Find references** |
| ![Navigating to a PureScript definition](.github/assets/vscode-demos/go-to-definition.gif) | ![Finding references to a PureScript name](.github/assets/vscode-demos/find-references.gif) |
| **Rename** | **Document symbols** |
| ![Renaming a PureScript name across files](.github/assets/vscode-demos/rename.gif) | ![Searching symbols in a PureScript document](.github/assets/vscode-demos/document-symbols.gif) |
| **Workspace symbols** | **Typed-hole suggestions** |
| ![Searching PureScript symbols across a workspace](.github/assets/vscode-demos/workspace-symbols.gif) | ![Replacing a typed hole with an Alexandrite suggestion](.github/assets/vscode-demos/typed-hole-suggestions.gif) |
| **Document highlights** | **Semantic highlighting** |
| ![Highlighting occurrences of PureScript names](.github/assets/vscode-demos/document-highlights.gif) | ![Enabling semantic highlighting for PureScript](.github/assets/vscode-demos/semantic-highlighting.gif) |

## Installation

On Linux and macOS:

```sh
curl --proto '=https' --tlsv1.2 -LsSf \
  https://raw.githubusercontent.com/purefunctor/purescript-alexandrite/main/install.sh | sh
```

On Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/purefunctor/purescript-alexandrite/main/install.ps1 | iex
```

The installers verify the release's GitHub build-provenance attestation when
[GitHub CLI](https://cli.github.com/) is available. They display a warning and continue when it is not
installed. Releases through v0.0.13 predate attestations, so the installers warn and continue without
verification for those versions. Set `ALEXANDRITE_VERSION` to a release tag or
`ALEXANDRITE_INSTALL_DIR` to an installation directory to override the defaults.
