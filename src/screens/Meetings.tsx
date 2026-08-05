import Placeholder from "./Placeholder";

export default function Meetings() {
  return (
    <Placeholder
      title="Meetings"
      day="Your meeting notes — captured on-device, never uploaded"
      intro="The archive and controls for the meeting listener. It sits quietly in the background, takes notes, and never intervenes."
      fields={[
        "Listener control — start / stop, the live listening state, on-device model status",
        "Meeting list — past meetings with title, date, duration, notes-only tag",
        "Meeting detail — summary, decisions, action items, and what you contributed",
        "Your contributions flag straight into the day's package and the vault",
        "Per-meeting: transcribe-and-discard, or retain the audio",
      ]}
      phase="Phase 7 · on-device audio"
    />
  );
}
