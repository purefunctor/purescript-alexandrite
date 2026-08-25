## Scope

The compiler backend turns checked frontend representations into executable JavaScript and validates
foreign JavaScript modules.

## Components

- **foreign-javascript**: parses foreign modules and validates their exports against PureScript declarations
- **nbe**: converts checked modules into owned functional trees for normalization by evaluation
- **javascript**: emits ES2022 JavaScript modules from functional trees

Preserve the direction `frontend → nbe → javascript`. Backend representations should be owned
and should not introduce dependencies from frontend crates back into code generation.

## Verification

- Run `cargo check -p <crate-name> --tests` for a changed crate.
- Run `cargo nextest run -p <crate-name>` for backend unit tests.
- Run `just t backend` whenever generated JavaScript, foreign-module validation, or executable
  behavior can change.
- Update generated fixture output only with `just t backend <filters> --update-output`, then inspect
  every changed file.

## Output Contracts

- Generated modules target ES2022 and Node.js 16 or newer.
- Report unsupported frontend states explicitly rather than silently emitting incorrect JavaScript.
- Keep pretty-printers useful for diagnosing intermediate functional trees.
