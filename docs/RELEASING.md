# Release process

The project uses one SemVer value in `VERSION`, the Cargo workspace, and the private Cypht Bridge. The release workflow accepts annotated `vMAJOR.MINOR.PATCH` tags and checks that the tag matches every version file.

## Prepare a release

1. Finish the release scope and all required runtime tests.
2. Set the product version with `python3 scripts/set-version.py X.Y.Z`.
3. Run the version, repository, PHP, Rust, frontend, OpenAPI, and integration checks.
4. Push the release commit to `main` and wait for both `CI` and `Container Images` to complete successfully for that exact commit.
5. Confirm the three authoritative full-SHA images exist for the commit.
6. Create and push an annotated tag only after those gates pass.

```bash
python3 scripts/set-version.py 0.5.0
python3 scripts/check-version.py
python3 scripts/check-repository.py
git tag -a v0.5.0 -m "Cypht Gateway v0.5.0"
git push origin v0.5.0
```

Do not run the tag commands until every release capability and deployment gate passes.

## Container publication model

`Container Images` runs only after a successful `CI` run on `main`. It checks out the exact CI `head_sha` and publishes, at most once, these immutable-by-policy identities:

```text
ghcr.io/<owner>/<repo>:sha-<full-40-char-git-sha>
ghcr.io/<owner>/<repo>-cypht-bridge:sha-<full-40-char-git-sha>
ghcr.io/<owner>/<repo>-aio:sha-<full-40-char-git-sha>
```

A rerun first inspects each full-SHA reference and does not overwrite an existing identity. After all three references exist, the workflow may move the convenience `edge` aliases when the tested commit is still the current `main` head.

The tag-triggered `Release` workflow is the only owner of release aliases. It does **not** rebuild container images. It verifies the exact full-SHA images produced from the release commit, builds the platform binaries, and only then promotes release aliases such as `v0.5.0`, `0.5.0`, `0.5`, and `latest` for a stable release. It also attaches `image-bundle.json` to the GitHub Release with the authoritative image refs and resolved digests.

Cross-package GHCR tag updates are coordinated but are not a registry transaction. Do not call them atomic. The full-SHA refs and `image-bundle.json` are the release identity if a mutable alias update is interrupted or a registry write fails.

Gateway images target AMD64 and ARM64. The Cypht Bridge `edge` image is built for AMD64 and ARM64 from the Cypht 2.13.0 baseline; the All-in-One image targets AMD64. Cypht 2.13.0 is the primary source-test baseline, while Cypht 2.12.0 remains a compatibility smoke baseline.

The first GHCR publication may be private. Set package visibility to public only if public pulls are intended. Keep Cypht credentials and Gateway secrets out of image build arguments.
