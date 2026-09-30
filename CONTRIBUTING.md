# Contributing to QuadQR SDK

Thanks for helping improve QuadQR. The JavaScript implementation is the v6 format reference, so changes to the wire format need special care.

## Before you start

- Use an issue or discussion to describe format changes, new public APIs, or larger platform work before implementing them.
- Keep changes focused. Explain the behavior, the reason for the change, and any compatibility effect in the pull request.
- Do not commit generated build output, local caches, native libraries, keys, credentials, or personal test images.
- By contributing, you agree that your contribution is licensed under this repository's AGPL-3.0-only license.

## Set up

Install stable Rust with rustup. For the desktop demo, use a graphical host OS and a webcam if you are changing live scanning. See the [README](README.md#requirements) for Node/WASM, Android, and Flutter prerequisites.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features -- -D warnings
cargo test --workspace --all-features --release
```

Run `cargo run --release -p quadqr-demo` for visual or camera changes. Use release builds when comparing scanner timing. The CI workflow runs core tests and demo compilation on Linux, macOS, and Windows.

## Platform checks

Run only the checks relevant to the code you changed:

| Area | Checks |
| --- | --- |
| Rust core or format | Workspace formatting, Clippy, and release tests; add a test for the changed behavior |
| JavaScript/WASM | From `packages/quadqr-js`: `npm run check:source`, `npm run build:wasm`, `npm run verify:wasm`, `npm run check` |
| Android JNI/Java/Kotlin | Build native libraries and the AAR with `build-android-release.*`; exercise the API on a supported ABI |
| Flutter | Run `build-flutter.*`, `flutter analyze`, and `flutter test` from the Flutter package; exercise the example on Android |
| Documentation | Install `scripts/requirements-docs.txt`, run `python scripts/build-doc-site.py` and `python scripts/check-doc-links.py`; check code examples and preview `documentation/index.html` at desktop and mobile widths |

If a toolchain or physical device is unavailable, say which check was skipped in the pull request. A real camera check is particularly useful for changes to finder detection, perspective recovery, color sampling, or frame scheduling.

## Compatibility rules

- Preserve v6 headers, matrix geometry, RGBW/Triangle16 mapping, masking, CRC-32, Reed–Solomon profiles, interleaving, and the compression/security envelopes unless a versioned format change is explicitly agreed.
- Keep browser camera and worker lifecycle in the JavaScript adapter; the reusable Rust core accepts matrices and image data.
- Add or update cross-language vectors for format changes. Verify both JS-generated data in Rust and Rust-generated data in the reference implementation when possible.
- Avoid introducing a new public API without updating its relevant guide and example.

## Pull request checklist

- Describe the user-visible effect and compatibility impact.
- Include meaningful tests or a reproducible manual check for behavior changes.
- Run the relevant checks above and report their results.
- Update README, package guides, or documentation site when setup, builds, or APIs change.
- Keep secrets and machine-specific paths out of commits and examples.

For security reports that should not be public, contact the maintainer privately through the repository's GitHub security advisory workflow.
