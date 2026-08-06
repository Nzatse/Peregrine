# Distribution — signing & notarization

Peregrine is designed to install with **zero prompts and no admin rights**. Getting there means clearing the two OS gates. The app is already user-space and telemetry-free; what's left is *signing*, which needs certificates only you can create (they're tied to your identity and cost money). This is the one step Claude can't do for you.

## The two gates (recap)

1. **Privilege (admin / UAC)** — already handled: Peregrine writes only to user space (config dir, keychain, its vault file). No services, drivers, or system changes. Nothing to elevate.
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

Then, on any platform:

```bash
npm ci
npm run tauri dev     # run it
npm run tauri build   # produce an installer
```

## CI

[`.github/workflows/build.yml`](../.github/workflows/build.yml) builds on **Windows and macOS**
runners (installs NASM/CMake as needed) on version tags or manual dispatch, and uploads the
installers as artifacts — this is what actually proves the app compiles on Windows. Add signing
secrets (below) to produce signed releases.



## macOS — Developer ID + notarization

**You need:** an [Apple Developer account](https://developer.apple.com) ($99/yr) and a **Developer ID Application** certificate.

**Then** set these before `npm run tauri build` (in CI: repo secrets):

```bash
export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAMID)"
export APPLE_ID="you@example.com"
export APPLE_PASSWORD="app-specific-password"   # appleid.apple.com → App-Specific Passwords
export APPLE_TEAM_ID="TEAMID"
```

Tauri signs the `.app`/`.dmg` and submits it for notarization automatically when these are set. The mic permission string is already in [`src-tauri/Info.plist`](../src-tauri/Info.plist).

> Note: `src-tauri/tauri.conf.json` → `bundle.macOS.signingIdentity` can pin the identity instead of the env var. Leave it unset to use `APPLE_SIGNING_IDENTITY`.

## Windows — Authenticode

**You need:** a code-signing certificate. [Azure Trusted Signing](https://learn.microsoft.com/azure/trusted-signing/) is the cheap modern option; an **EV certificate** clears SmartScreen instantly.

Configure the signing command / thumbprint in `tauri.conf.json` → `bundle.windows.certificateThumbprint` (or use `tauri-action`'s Windows signing inputs in CI).

## CI (optional, recommended)

Build signed artifacts on every tag with [`tauri-apps/tauri-action`](https://github.com/tauri-apps/tauri-action): it runs `tauri build` on macOS + Windows runners and uploads the installers. Put the certs/passwords above into repository **Secrets** — never commit them.

## What this buys

A signed, notarized, user-space app that any professional can download and run **with no warning and no admin password**, even on a locked-down work laptop — the exact promise in the README. Total cost of admission: ~$100/yr for the Apple account plus a Windows cert.
