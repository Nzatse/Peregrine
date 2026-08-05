import Placeholder from "./Placeholder";

export default function Memory() {
  return (
    <Placeholder
      title="Memory"
      day="Your curated career vault — outcomes, not raw events"
      intro="The polished record you'd actually show someone. Where Timeline is when things happened, Memory is the quantified accomplishments they add up to."
      fields={[
        "Accomplishments — framed, quantified wins",
        "Grouped by project, skill, or theme",
        "Each entry: the outcome, its metric, and provenance back to the signals that produced it",
        "Edit, and a needs-metric flag for anything still missing its number",
        "Search — find anything by keyword or skill",
      ]}
      phase="Phase 4"
    />
  );
}
