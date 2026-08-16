# CLAUDE.md

Guidance for Claude Code when working in this repository.

## Language

dev-forge is an open source project, so **everything written into the
repository or published from it is in English**:

- Commit messages
- Issue titles and bodies
- Pull request titles, bodies, and review comments
- GitHub Releases and release notes
- Code comments, doc comments, and identifiers
- README, `docs/`, and every other Markdown file

Conversation with the maintainer stays in Japanese. Only the artifacts above
are English.

Commits made before this policy are in Japanese. Leave them alone — history is
not rewritten for this.

## Conventions

- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/)
  in the imperative mood, e.g. `feat(jwt): print exp as a datetime`. Common
  scopes: `repl`, `picker`, `banner`, `base64`, `url`, `jwt`, `timestamp`,
  `readme`, `release`.
- Before pushing: `cargo clippy --all-targets -- -D warnings` and `cargo test`,
  both clean. New lines are expected to be 80% covered — that is the Codecov
  patch target and the check most likely to fail.
- The tree is not rustfmt-clean; do not run `cargo fmt` over it as part of an
  unrelated change. Match the surrounding style instead.
- Tests live in a `#[cfg(test)] mod tests` at the bottom of the file they
  cover.
- A user-visible change updates `docs/` in the same commit.

See [CONTRIBUTING.md](CONTRIBUTING.md) for the build, test, and release details.
