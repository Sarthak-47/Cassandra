# Contributing to Cassandra

Thanks for considering a contribution. This document is required policy
for pull requests (see `.github/copilot-instructions.md` and
`.github/PULL_REQUEST_TEMPLATE.md`) — PRs that don't follow it will get
blocking feedback, not just suggestions.

## Before you start

- For anything beyond a small fix, open an issue first describing what
  you want to change and why. Saves everyone a rewritten PR.
- Check [ROADMAP.md](ROADMAP.md) and [REALIGNMENT/](REALIGNMENT/INDEX.md)
  — if what you want to build overlaps with the compression-engine
  rewrite in progress there, say so in your issue before writing code
  against the architecture that's about to change.

## Development setup

This is a mixed Rust/Python workspace built with
[maturin](https://www.maturin.rs/).

```bash
python -m pip install maturin
pip install -e ".[dev]"       # or a more specific extra: .[proxy,dev], .[dev,agno], etc.
cargo check --workspace       # Rust side
```

## Making changes

- **Commit messages** follow [Conventional Commits](https://www.conventionalcommits.org/)
  (enforced by commitlint on PRs — see `.commitlintrc.json` for the exact
  allowed types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`,
  `test`, `build`, `ci`, `deps`, `chore`, `revert`). The type determines
  the automated release version bump, so get it right:
  - `fix:` — bug fix (patch bump)
  - `feat:` — new user-facing capability (minor bump)
  - anything else — no version bump
- **Tests**: add or update tests for anything you change. Run the
  relevant subset locally before opening a PR:
  ```bash
  pytest tests/                     # Python
  cargo test --workspace            # Rust
  ```
- **Lint/format** must be clean:
  ```bash
  ruff check .
  ruff format --check .
  mypy cassandra --ignore-missing-imports
  cargo clippy --workspace
  ```

## Pull requests

Fill out the PR template completely, especially the **Real Behavior
Proof** section — environment, exact commands you ran, and what you
actually observed. "Should work" or an empty section gets the PR sent
back. If something wasn't tested, say so explicitly rather than leaving
it implied.

For user-facing, release, dependency, workflow, or security-sensitive
changes, expect stricter review — these are exactly the categories
`.github/copilot-instructions.md` tells reviewers to be strict about.

## Security issues

Do not open a public issue for a security vulnerability. See
[SECURITY.md](SECURITY.md).

## Code of conduct

This project follows [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## License

By contributing, you agree your contributions are licensed under the
project's [Apache 2.0 license](LICENSE).
