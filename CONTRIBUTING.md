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

Tags are annotated and formatted as `vMAJOR.MINOR.PATCH`. Follow `docs/RELEASING.md` after the release scope and all gates pass. A dirty development tree may sit on an older tag. The version check warns in that case, while tagged CI still requires an exact version match.
