import { useState } from "react";
import { Falcon } from "../components/icons";
import { createVault, unlockVault } from "../api";

export default function VaultGate({
  kind,
  onDone,
}: {
  kind: "onboard" | "unlock";
  onDone: () => void;
}) {
  const [pass, setPass] = useState("");
  const [confirm, setConfirm] = useState("");
  const [err, setErr] = useState("");
  const [busy, setBusy] = useState(false);
  const onboarding = kind === "onboard";

  async function submit() {
    setErr("");
    if (onboarding && pass !== confirm) {
      setErr("Passphrases don't match.");
      return;
    }
    setBusy(true);
    try {
      if (onboarding) await createVault(pass);
      else await unlockVault(pass);
      onDone();
    } catch (e) {
      setErr(String(e));
      setBusy(false);
    }
  }

  return (
    <div className="gate">
      <div className="gate-card">
        <div className="gate-mark"><Falcon /></div>
        <div className="gate-tag">making chaos make sense</div>
        <h1>{onboarding ? "Create your career vault" : "Unlock your vault"}</h1>
        <p>
          {onboarding
            ? "Your work stays in a single encrypted file that only you can open. Choose a passphrase — it's the only key, and there's no way to recover it, so keep it safe."
            : "Enter your passphrase to open your career vault."}
        </p>

        <input
          className="field mono"
          type="password"
          autoFocus
          placeholder={onboarding ? "Choose a passphrase" : "Passphrase"}
          value={pass}
          onChange={(e) => setPass(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && !onboarding && submit()}
        />
        {onboarding && (
          <input
            className="field mono"
            type="password"
            placeholder="Confirm passphrase"
            value={confirm}
            onChange={(e) => setConfirm(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && submit()}
          />
        )}

        {err && <div className="gate-err">{err}</div>}

        <button className="btn primary gate-go" onClick={submit} disabled={busy || !pass}>
          {busy ? "Working…" : onboarding ? "Create vault" : "Unlock"}
        </button>

        {onboarding && (
          <div className="gate-note">Encrypted with SQLCipher · stored on this machine · never uploaded</div>
        )}
      </div>
    </div>
  );
}
