# Distribution — signing & notarization

Peregrine is designed to install with **zero prompts and no admin rights**. Getting there means clearing the two OS gates. The app is already user-space and telemetry-free; what's left is *signing*, which needs certificates only you can create (they're tied to your identity and cost money). This is the one step Claude can't do for you.

## The two gates (recap)

1. **Privilege (admin / UAC)** — already handled: Peregrine writes only to user space (config dir, its encrypted vault file). No services, drivers, or system changes. Nothing to elevate.
2. **"Unidentified developer" (Gatekeeper / SmartScreen)** — this is what signing fixes.

### What a user sees opening a GitHub download

- **Never an admin / UAC password prompt.** Peregrine installs and runs entirely in user space.
- **Unsigned** build: a one-time warning — macOS "unidentified developer" (right-click → Open, or Settings → Privacy → Open Anyway), Windows SmartScreen ("More info" → Run anyway). Dismissable, *not* admin.
- **Signed + notarized** build: nothing at all — it just opens.
- The meeting listener asks a normal per-app **microphone** permission on first use (not admin).

## Building from source

End users of a released build need none of this — only whoever compiles it. Peregrine
bundles SQLCipher, OpenSSL, and whisper.cpp from source (that's what keeps it
self-contained and private), so the build host needs a C/C++ toolchain.

**All platforms:** [Node](https://nodejs.org) 20+, [Rust](https://rustup.rs), and [CMake](https://cmake.org) (for whisper.cpp).

**macOS:** Xcode Command Line Tools (`xcode-select --install`) + `brew install cmake`. Perl is preinstalled.

**Windows:**
- Rust with the **MSVC** toolchain
- **Visual Studio Build Tools** (C++ workload)
- **CMake**
- **Strawberry Perl** and **NASM** — required by the vendored OpenSSL build (`choco install nasm strawberryperl`)

**Linux (Debian/Ubuntu):**

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev \
  patchelf libxdo-dev libasound2-dev build-essential cmake
```

(`libasound2-dev` is ALSA — the meeting listener needs it.)

Then, on any platform:

```bash
npm ci
npm run tauri dev     # run it
npm run tauri build   # produce an installer
```

## Releasing

[`.github/workflows/build.yml`](../.github/workflows/build.yml) runs
[`tauri-apps/tauri-action`](https://github.com/tauri-apps/tauri-action) on every `v*` tag across
four targets — macOS `aarch64` + `x86_64`, Windows `x86_64`, Linux `x86_64` — and publishes the
installers to a **draft** GitHub Release along with `latest.json`, the manifest the in-app updater
reads. Manual dispatch runs the same build but only uploads workflow artifacts, so it stays a
"does it still bundle?" check.

The bundle targets in `tauri.conf.json` (`app`, `dmg`, `nsis`, `msi`, `deb`, `appimage`) are
exactly what those runners produce. `rpm` is deliberately excluded — it would need `rpmbuild` on
the Linux runner and we don't ship it.

To cut a release:

```bash
npm version 0.1.1 --no-git-tag-version     # bump package.json
# match the version in src-tauri/tauri.conf.json and src-tauri/Cargo.toml
git commit -am "Release v0.1.1" && git push
git tag v0.1.1 && git push origin v0.1.1
```

Then review the draft release on GitHub and hit **Publish**.

## Auto-updates

Peregrine checks for updates only when the user asks (**Settings → Updates → Check**) — no
background polling, consistent with the no-telemetry promise. The update is fetched and its
signature verified in Rust, so the webview CSP stays closed.

Updates only install if the release was built with the updater signing key. That, plus the
per-OS code-signing certificates, is covered in **[SIGNING.md](./SIGNING.md)** — the exact
certs to buy, how to export them, and which repository secrets to add.

## What this buys

A signed, notarized, user-space app that any professional can download and run **with no warning and no admin password**, even on a locked-down work laptop — the exact promise in the README. Total cost of admission: ~$100/yr for the Apple account plus a Windows cert.
