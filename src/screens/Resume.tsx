import Placeholder from "./Placeholder";

export default function Resume() {
  return (
    <Placeholder
      title="Résumé"
      day="Render your memory into artifacts — you review, you publish"
      intro="Turns your vault into résumé bullets and more, built on top of your existing résumé. Peregrine drafts; you always ship."
      fields={[
        "Import your existing résumé as the baseline it builds on",
        "Generated bullets — quantified, outcome-first, each with provenance and edit",
        "Tailor to a job — paste a description, it selects and reframes",
        "Other renders — LinkedIn about, self-review, promo packet, weekly summary",
        "Export only — never auto-posts anywhere",
      ]}
      phase="Phase 6"
    />
  );
}
