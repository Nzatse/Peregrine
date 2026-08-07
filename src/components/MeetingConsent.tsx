import { Mic } from "./icons";

export const CONSENT_KEY = "peregrine-meeting-consent";

export default function MeetingConsent({
  onAccept,
  onCancel,
}: {
  onAccept: () => void;
  onCancel: () => void;
}) {
  return (
    <div className="overlay" role="dialog" aria-modal="true" aria-label="Recording consent">
      <div className="gate-card" style={{ maxWidth: 460 }}>
        <div className="gate-mark"><Mic /></div>
        <h1>Before you record</h1>
        <p>
          Recording a meeting captures <b>everyone you can hear — including the other people on the
          call</b>, not just you.
        </p>
        <ul className="consent-list">
          <li>Make sure the others know they're being recorded and are okay with it.</li>
          <li>
            Recording-consent laws vary by location — some require <b>everyone's</b> consent.
            Complying is your responsibility.
          </li>
          <li>Audio is transcribed <b>on your device</b> and never uploaded — only the notes are kept.</li>
        </ul>
        <button className="btn primary gate-go" onClick={onAccept}>I understand — I have consent</button>
        <button className="btn" onClick={onCancel}>Cancel</button>
      </div>
    </div>
  );
}
