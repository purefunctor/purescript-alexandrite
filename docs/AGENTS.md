## Playground boundary

This documentation site provides a browser-based playground for PureScript type checking, parsing,
and package loading through the WASM compiler.

Keep compiler operations in the Web Worker so compilation does not block the UI. The WASM interface
adapts compiler results for the playground; language semantics belong in the compiler crates rather
than React components or worker glue.

Use `package.json` for the site's development, build, type-checking, and formatting commands.
