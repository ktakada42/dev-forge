# Contributing to dev-forge

Thanks for taking the time. Bug reports, ideas for new tools, and pull requests
are all welcome.

## Language

**Everything written in this repository is in English**: commit messages,
issues, pull requests, release notes, code comments, and documentation. The
project is open source and English is the language its readers have in common.

Discussion in an issue thread can happen in whatever language the participants
share, but anything that lands in the repository or in a release must be
English.

## Getting started

You need a [Rust](https://rustup.rs) toolchain (stable). Nothing else.

```sh
git clone https://github.com/ktakada42/dev-forge
cd dev-forge
cargo build
cargo run          # starts the REPL
cargo run -- base64 encode hello
```

The binary is `forge`; the crate is `dev-forge`.

## Before you push

```sh
cargo clippy --all-targets -- -D warnings
cargo test
```

Both are expected to pass clean. On formatting: the tree is not rustfmt-clean
today, so running `cargo fmt` over the whole crate would bury your change in
unrelated reformatting. Match the style of the code around you, and keep any
reformatting out of a PR that also changes behavior.

CI runs the test suite with coverage on every pull request and uploads it to
Codecov. New or changed lines are expected to be **80% covered** — that is the
patch target in [`codecov.yml`](codecov.yml), and it is the one check that
tends to fail on an otherwise fine PR. Tests live in `#[cfg(test)] mod tests`
at the bottom of the file they cover.

To see coverage the way CI does:

```sh
cargo install cargo-llvm-cov     # once
cargo llvm-cov --html --open
```

## Project layout

| Path | What lives there |
| --- | --- |
| `src/main.rs` | CLI definition (clap) and the subcommand dispatch |
| `src/repl.rs` | Interactive mode: the tool/direction questions and the input loop |
| `src/picker.rs` | The list widget the questions are asked with |
| `src/banner.rs` | The startup animation |
| `src/tools/` | The conversions themselves — one module per tool |
| `docs/` | User documentation linked from the README |
| `assets/demo.tape` | The [VHS](https://github.com/charmbracelet/vhs) script that records `demo.gif` |

### Adding a tool

A tool is a module in `src/tools/` exposing plain functions over `&str`, and
that module is then wired into both front ends:

1. `src/tools/<name>.rs` — the conversion, plus its unit tests.
2. `src/tools/mod.rs` — declare the module.
3. `src/main.rs` — a `Commands` variant, so it works in pipes.
4. `src/repl.rs` — a `Tool` variant, a row in `TOOLS`, its entry in
   `TOOL_ORDER`, and an arm in `choose_mode`, `convert`, `prompt`, and
   `print_intro`.
5. `docs/tools/<name>.md` and a row in the README table.

Keep the interactive name and the subcommand name identical — that equivalence
is the point of having both.

## Commits

[Conventional Commits](https://www.conventionalcommits.org/), in English, in
the imperative mood:

```
feat(jwt): print exp and iat as human-readable datetimes
fix(timestamp): keep the fraction when converting millisecond input
docs(readme): restructure into an overview plus linked guides
ci(release): publish to crates.io on tag
```

Common scopes are the module or area touched: `repl`, `picker`, `banner`,
`base64`, `url`, `jwt`, `timestamp`, `readme`, `release`.

Commits made before this policy are in Japanese; they are history and stay as
they are.

## Pull requests

1. Branch off `main` — `feat/…`, `fix/…`, `docs/…`, `ci/…`.
2. Make the change, with tests for anything a user could hit.
3. Run the commands under [Before you push](#before-you-push).
4. Open the PR against `main`. Describe what changes and why; if behavior
   changes, paste the before and after.
5. Update the docs in the same PR. A flag with no line in `docs/` is unfinished.

Keep one PR to one concern. Small and reviewable beats complete and enormous.

## Reporting bugs

Open an [issue](https://github.com/ktakada42/dev-forge/issues) with:

- `forge --version`, your OS, and — for anything about the interactive mode —
  your terminal emulator, since key handling differs between them.
- The exact input, the output you got, and the output you expected.

## Releases

Maintainers only. Pushing a `v*` tag runs
[`.github/workflows/release.yml`](.github/workflows/release.yml), which builds
the macOS and Linux binaries, creates the GitHub release with generated notes,
publishes the crate to [crates.io](https://crates.io/crates/dev-forge), updates
the Homebrew formula in `ktakada42/homebrew-tap`, and commits the version bump
back to `main`.

```sh
git tag v2.1.0
git push origin v2.1.0
```

Because the release notes are generated from commit subjects, the English rule
above is what keeps them readable.

The crates.io step authenticates with
[trusted publishing](https://crates.io/docs/trusted-publishing): the job trades
a GitHub OIDC token for a short-lived registry token, so there is no API token
stored as a repository secret. crates.io grants that trade only to
`release.yml` on this repository, which is why moving or renaming the workflow
means updating the trusted publisher on the crate's settings page.

## License

By contributing, you agree that your contributions are licensed under the
[MIT License](LICENSE).
