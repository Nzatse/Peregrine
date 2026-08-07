# Signing & release secrets

Everything in this file is something **only Henri can do** — the certificates are
tied to a legal identity and cost money. The build works without any of it (the
installers are just unsigned), so treat this as a checklist you work through
once, not a blocker.

There are **two independent kinds of signing**. They are easy to confuse:

| | What it proves | Needed for | Cost |
|---|---|---|---|
| **Updater signing** (`TAURI_SIGNING_*`) | This update came from Peregrine | Auto-updates to install at all | Free |
| **OS code signing** (Apple / Windows) | The OS trusts this app | No Gatekeeper / SmartScreen warning | ~$99/yr + Windows cert |

Where every secret goes: **GitHub → your repo → Settings → Secrets and variables
→ Actions → *New repository secret***
(`https://github.com/Nzatse/Peregrine/settings/secrets/actions`).

---

## 1. Updater signing — free, do this first

Without these two secrets, `tauri-action` still builds installers, but it cannot
produce the signed `latest.json` the in-app updater requires, so **updates will
silently never install**.

The keypair has already been generated. The public half is committed in
`src-tauri/tauri.conf.json` → `plugins.updater.pubkey`. The private half was
printed to you once and is **not** in this repo — it must never be.

| Secret | Value |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | The contents of the private key file (the long base64 blob) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | The password the key was generated with |

> **If you lose the private key or its password**, you cannot ship updates to
> anyone already running Peregrine. You would have to generate a new keypair,
> ship a new public key, and every existing user would have to reinstall by
> hand. Put both in a password manager now.

To regenerate from scratch (only if you must):

```bash
npm run tauri signer generate -- -w ~/.tauri/peregrine.key
# then paste the new PUBLIC key into src-tauri/tauri.conf.json → plugins.updater.pubkey
```

---

## 2. macOS — Developer ID + notarization

### What to buy

An [Apple Developer Program](https://developer.apple.com/programs/) membership,
**$99/yr**. Enrollment can take a day or two to be approved.

### Create the certificate

1. Xcode → **Settings → Accounts** → sign in → select your team → **Manage Certificates**
2. **+** → **Developer ID Application**
   *(Not "Apple Development" and not "Developer ID Installer" — those are different certs and will not clear Gatekeeper for a distributed `.app`.)*
3. Confirm it landed: `security find-identity -v -p codesigning`
   You want the line reading `Developer ID Application: Your Name (TEAMID)`.

### Export it for CI

CI runners have no keychain, so the cert goes in as a base64 `.p12`:

1. **Keychain Access** → *My Certificates* → right-click the *Developer ID
   Application* cert → **Export…** → save as `.p12`, set a strong password.
2. Base64 it:
   ```bash
   base64 -i certificate.p12 | pbcopy
   ```
3. Delete the `.p12` from disk afterwards — it is a private key.

### App-specific password for notarization

Apple will not accept your account password. Go to
[appleid.apple.com](https://appleid.apple.com) → **Sign-In and Security** →
**App-Specific Passwords** → generate one (format `abcd-efgh-ijkl-mnop`).

Your **Team ID** is the 10-character code at
[developer.apple.com/account](https://developer.apple.com/account) → Membership.

### Secrets to add

| Secret | Example / where it comes from |
|---|---|
| `APPLE_CERTIFICATE` | base64 of the `.p12` (step 2 above) |
| `APPLE_CERTIFICATE_PASSWORD` | the password you set when exporting the `.p12` |
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: Henri Nzatse (AB12CD34EF)` |
| `APPLE_ID` | the Apple ID email on the developer account |
| `APPLE_PASSWORD` | the app-specific password |
| `APPLE_TEAM_ID` | `AB12CD34EF` |

All six are already referenced in `.github/workflows/build.yml`. Once they
exist, the next tag produces a signed **and notarized** `.dmg` — `tauri-action`
imports the cert, signs, submits to Apple, and staples the ticket with no
further changes.

Verify a downloaded build:

```bash
spctl -a -vvv -t install /Applications/Peregrine.app   # → "accepted / Notarized Developer ID"
xcrun stapler validate /Applications/Peregrine.app
```

The microphone usage string notarization requires is already in
`src-tauri/Info.plist`.

---

## 3. Windows — Authenticode

Pick **one** of the two routes.

### Route A — Azure Trusted Signing (recommended)

~**$10/month**, no hardware token, and it works in cloud CI. The catch: your
organisation must have been **verified for 3+ years** (individual developers
have a separate, newer identity-validation path).

1. In the Azure portal, create a **Trusted Signing account** and a **Certificate
   Profile**; complete identity validation.
2. Create an **App Registration** (service principal) and grant it the
   **Trusted Signing Certificate Profile Signer** role on the account.
3. Add to `src-tauri/tauri.conf.json`:
   ```json
   "bundle": {
     "windows": {
       "signCommand": "trusted-signing-cli -e https://eus.codesigning.azure.net -a MyAccount -c MyProfile %1"
     }
   }
   ```
4. Install the signer on the Windows runner, before the `tauri-action` step in
   `.github/workflows/build.yml`:
   ```yaml
   - name: Install trusted-signing-cli (Windows)
     if: runner.os == 'Windows'
     run: cargo install trusted-signing-cli
   ```
5. Add these three to the `env:` block of the `tauri-action` step:
   ```yaml
   AZURE_CLIENT_ID: ${{ secrets.AZURE_CLIENT_ID }}
   AZURE_CLIENT_SECRET: ${{ secrets.AZURE_CLIENT_SECRET }}
   AZURE_TENANT_ID: ${{ secrets.AZURE_TENANT_ID }}
   ```

| Secret | From |
|---|---|
| `AZURE_CLIENT_ID` | App Registration → Application (client) ID |
| `AZURE_CLIENT_SECRET` | App Registration → Certificates & secrets |
| `AZURE_TENANT_ID` | App Registration → Directory (tenant) ID |

### Route B — EV / OV certificate from a CA

DigiCert, Sectigo, SSL.com — roughly **$250–$600/yr**. An **EV** cert clears
SmartScreen immediately; a plain **OV** cert still accrues reputation over time
(users may see SmartScreen for the first few hundred downloads).

The important constraint: since June 2023 the private key must live on a
**FIPS 140-2 HSM**. That means either:

- **A physical USB token** — cannot be used from GitHub-hosted runners at all.
  You would sign locally on your own Windows machine and upload the installer to
  the release by hand.
- **A cloud signing service** (DigiCert KeyLocker, SSL.com eSigner) — works in
  CI, wired the same way as Route A: set `bundle.windows.signCommand` to the
  vendor's CLI and pass its credentials as repo secrets.

If you use a local certificate store instead, set
`bundle.windows.certificateThumbprint` in `tauri.conf.json` and sign on your own
machine.

Verify a signed installer:

```powershell
Get-AuthenticodeSignature .\Peregrine_0.1.0_x64-setup.exe | Format-List
```

---

## 4. Linux

Nothing to buy. `.deb` and `.AppImage` are unsigned by convention; the AppImage
still carries the Tauri updater signature from §1, which is what matters for
auto-updates.

---

## What happens with no secrets at all

The workflow still succeeds. You get unsigned installers on the release, and
users see a one-time, dismissable warning (macOS: right-click → Open; Windows
SmartScreen: More info → Run anyway). Auto-updates will not install until §1 is
done. See [DISTRIBUTION.md](./DISTRIBUTION.md) for what users actually see.
