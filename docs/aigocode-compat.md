# CC Switch AIGoCode compatibility build

This is a community build of [CC Switch](https://github.com/farion1231/cc-switch) with an AIGoCode provider-import compatibility patch. The upstream authors retain their attribution and the project remains distributed under the [MIT license](../LICENSE). This build is maintained at [jupiterchu/cc-switch](https://github.com/jupiterchu/cc-switch); it is not an official CC Switch release.

## Downloads and installation

The first compatibility revision uses tag `aigocode-v3.20.0-1` and keeps application version `3.20.0`.

- Windows x64: `CC-Switch-aigocode-v3.20.0-1-Windows-x64.msi`. Close CC Switch and run the MSI. The installer does not have an Authenticode signature.
- macOS 12 or later, Intel and Apple Silicon: `CC-Switch-aigocode-v3.20.0-1-macOS-universal.zip`. Close CC Switch, extract the app, and move it to Applications. The app has an ad-hoc signature and is **not Apple notarized**. macOS may require approval in System Settings > Privacy & Security before opening it.
- Verify downloads against the accompanying `SHA256SUMS.txt`. The original MIT license and this document are included in the application resources and release assets.

The application name, identifier `com.ccswitch.desktop`, and `ccswitch://` URL scheme are retained for an in-place replacement. This is not a separate side-by-side installation. Existing settings and the CC Switch database are reused; back up data with the existing application before replacing it. The Windows installer already allows same-version upgrades. Installing an upstream package later can replace the compatibility patch.

## Updates

This compatibility build does not automatically check for, download, or install application updates. “Check Updates,” release notes, and recovery download links lead to this fork's [Releases page](https://github.com/jupiterchu/cc-switch/releases), where users install a reviewed compatibility build manually.

Both layers are required: the Tauri overlay disables updater artifacts and clears all updater endpoints; the frontend and Rust compatibility guards disable update operations. Ordinary builds without the compatibility flags retain upstream update behavior. Tool updates inside CC Switch are unaffected.

A future automatic update channel would need this fork's own signing key and public key, endpoints, signed artifacts, and update manifest. Do not reuse the upstream endpoints or claim to sign updates with the upstream key.

## Build and draft release

Use the `AIGoCode compatibility build` workflow in `jupiterchu/cc-switch`. It is triggered only with `workflow_dispatch`; its `tag` input defaults to `aigocode-v3.20.0-1`. The tag must match the application version and end in a positive revision number. This tag prefix does not match the upstream release workflow's `v*` trigger.

The workflow builds the exact dispatched commit on Windows and macOS, uploads both artifacts, computes checksums, and creates a **draft prerelease**. It does not publish the release. The workflow refuses to build in the upstream repository and refuses to reuse a tag pointing to a different commit. An existing release is not overwritten; use a new revision for changed artifacts.

Build inputs:

- Node.js 22, pnpm from `packageManager`, Rust 1.95.0.
- `src-tauri/tauri.aigocode.conf.json` merged over the ordinary platform configuration.
- Frontend: `VITE_AIGOCODE_COMPAT=1` and `VITE_AIGOCODE_RELEASES_URL=https://github.com/jupiterchu/cc-switch/releases`.
- Rust: `AIGOCODE_COMPAT_BUILD=1` and `AIGOCODE_RELEASES_URL=https://github.com/jupiterchu/cc-switch/releases`.

For local builds, set the same four environment variables before running:

```sh
pnpm install --frozen-lockfile
# On Windows x64:
pnpm tauri build --config src-tauri/tauri.aigocode.conf.json --target x86_64-pc-windows-msvc --bundles msi
# On macOS, after installing both Rust targets:
rustup target add aarch64-apple-darwin x86_64-apple-darwin
pnpm tauri build --config src-tauri/tauri.aigocode.conf.json --target universal-apple-darwin --bundles app
```

No Apple, Windows, Tauri updater, or R2 signing credentials are required by this workflow. GitHub's automatically supplied token needs `contents: write` only in the draft-release job. The original release workflow cannot be reused without its Tauri signing key and Apple signing/notarization credentials.

Before publishing the draft, verify installation and the AIGoCode provider import on Windows and macOS, confirm existing configuration is retained, and confirm “Check Updates” opens this fork's Releases page. The workflow runs the `deeplink_import` integration suite on native Windows x64 and macOS arm64, covering provider import, local configuration writing, and switching. It also verifies the macOS ad-hoc signature and both CPU architectures. These checks do not substitute for a GUI installation check; no GUI installation result is claimed by this document.
