# Contributing

## Branches

- `main` must remain releasable.
- Use short-lived feature branches.
- Changes should enter `main` through pull requests when the project is hosted on GitHub.

## Commit format

Use Conventional Commits:

```text
feat: add message search API
fix: preserve IMAP UIDVALIDITY
refactor: split Cypht adapter
ci: build arm64 image
chore: bump dependencies
```

Breaking changes use `!` or a `BREAKING CHANGE:` footer.

## Versioning

The product follows SemVer. `VERSION` and the Cargo workspace version must match. Run:

```bash
python3 scripts/check-version.py
```

before committing a release change.

Tags are formatted as `vMAJOR.MINOR.PATCH`.
