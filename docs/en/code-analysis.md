# Code analysis

> Section: [Documentation](./README.md)

Dev Manager parses source via **tree-sitter** and provides:
- symbol extraction (functions/classes/structs) with signatures and doc comments;
- linters: DRY, KISS, duplicates, unused code;
- the basis for `dm commit auto`.

## Running

```sh
dm lint            # every service
dm lint api        # only the api service
```

Enabled checks come from the `linter:` section of `dm.yaml`:
```yaml
linter:
  dr: true
  kiss: true
  unused_code: true
  duplicates: true
  auto_fix: false
```

## Finding categories

### `duplicate` — duplicate definitions
A symbol with the same `(name, category)` is defined in **different files**.
Often from copy-paste or accidental name collisions in a monorepo.

### `dry` — DRY violation
A function with the same name appears in several files — a likely
copy-paste duplicate.

### `kiss` — KISS violation
- A function with more than 6 parameters — consider grouping into a struct.
- A function longer than 80 lines — consider splitting.

Thresholds are conservative and configurable in source
(`crates/dm-analysis/src/lints/kiss.rs`).

### `unused` — unused code
A symbol's name appears in the project only in its own definition (fewer than 2
occurrences across all files). This is a heuristic without a call graph — simple
but effective; possible false positives for dynamic dispatch.

## Supported languages

In this version: **Rust, JavaScript, TypeScript, Go**.

Adding a language means implementing the `LanguageParser` trait (in
`crates/dm-analysis/src/languages/`) and registering it in
`parser_for_extension`. Any tree-sitter grammar plugs in the same way.

## Doc comments

Recognized:
- Rust: `///`, `//!`, `/** */`;
- JS/TS: JSDoc `/** ... */`, `//`;
- Go: `//` above a definition.

The first paragraph of a doc comment is used in `commit auto` and reports.

## `dm commit auto`

Uses the same tree-sitter engine: for each changed file, `dm` takes the
`git HEAD` version and the on-disk version, compares symbols and builds a
message with the change type (`added`/`modified`/`removed`).
