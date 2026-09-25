# Release process

The project uses one SemVer value in `VERSION`, the Cargo workspace, and the private Cypht Bridge. The release workflow accepts annotated `vMAJOR.MINOR.PATCH` tags and checks that the tag matches every version file.

## Prepare a release

1. Finish the release scope and all required runtime tests.
2. Set the product version with `python3 scripts/set-version.py X.Y.Z`.
3. Run the version, repository, PHP, Rust, frontend, OpenAPI, and integration checks.
4. Commit the version files and release notes on a clean branch.
5. Create and push an annotated tag only after the commit is ready.

```bash
python3 scripts/set-version.py 0.5.0
python3 scripts/check-version.py
python3 scripts/check-repository.py
git tag -a v0.5.0 -m "Cypht Gateway v0.5.0"
git push origin v0.5.0
```

Do not run the tag commands for v0.5.0 until every v0.5 capability and deployment gate passes. The tag workflow builds platform binaries, creates Git, source, and Bridge ZIP files, publishes versioned Gateway and Cypht Bridge images to GHCR, and writes `SHA256SUMS`. It tags the Gateway image as `ghcr.io/<owner>/<repo>:<version>` and the Bridge image as `ghcr.io/<owner>/<repo>-cypht-bridge:<version>`. Gateway images target AMD64 and ARM64. The pinned Cypht 2.12.0 Bridge image targets the verified Unraid AMD64 host.

The Gateway image uses `ghcr.io/<owner>/<repo>:<version>`. The Bridge image uses `ghcr.io/<owner>/<repo>-cypht-bridge:<version>`. The workflow also publishes `latest` for stable tags. It builds Linux AMD64 and ARM64 images.

The first GHCR publication may be private. Set package visibility to public only if public pulls are intended. Keep Cypht credentials and Gateway secrets out of image build arguments.

