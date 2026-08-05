import Placeholder from "./Placeholder";

export default function Debrief() {
  return (
    <Placeholder
      title="Debrief"
      day="The nightly interview that fills the gaps while they're fresh"
      intro="At the end of the day, Peregrine reviews your package, finds the unfinished stories, and interviews you to complete them."
      fields={[
        "Day review — the package with unfinished stories flagged",
        "Interview — one focused question at a time (the missing metric, the outcome, the context)",
        "How to find out — coaching when you can't answer (check the ticket, ask a teammate)",
        "The skill you demonstrated, named — the moment it improves you",
        "Completion — resolved stories filed into Memory",
      ]}
      phase="Phase 5"
    />
  );
}
