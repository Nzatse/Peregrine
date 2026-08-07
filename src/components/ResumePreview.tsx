import { type ResumeDoc, type SectionKey, stripCites } from "../resume";

// A clean, print-ready rendering of the structured résumé. Citations are stripped
// here — they exist for grounding review in the editor, not in the finished doc.
export default function ResumePreview({ doc }: { doc: ResumeDoc }) {
  const p = doc.profile;
  const contact = [p.email, p.phone, p.location, p.website, p.linkedin, p.github].filter(Boolean);

  const section = (k: SectionKey) => {
    if (k === "summary") {
      if (!doc.summary.trim()) return null;
      return (
        <section className="rp-sec" key={k}>
          <h2>Summary</h2>
          <p className="rp-summary">{stripCites(doc.summary)}</p>
        </section>
      );
    }
    if (k === "experience") {
      const items = doc.experience.filter((x) => x.role || x.company || x.bullets.some(Boolean));
      if (!items.length) return null;
      return (
        <section className="rp-sec" key={k}>
          <h2>Experience</h2>
          {items.map((x, i) => (
            <div className="rp-entry" key={i}>
              <div className="rp-entry-h">
                <span className="rp-role">{[x.role, x.company].filter(Boolean).join(", ")}</span>
                <span className="rp-meta">{[x.location, x.date].filter(Boolean).join(" · ")}</span>
              </div>
              <ul>
                {x.bullets.filter((b) => b.trim()).map((b, j) => <li key={j}>{stripCites(b)}</li>)}
              </ul>
              {x.tools.length > 0 && <div className="rp-tools">{x.tools.join(" · ")}</div>}
            </div>
          ))}
        </section>
      );
    }
    if (k === "skills") {
      const items = doc.skills.filter((s) => s.items.length);
      if (!items.length) return null;
      return (
        <section className="rp-sec" key={k}>
          <h2>Skills</h2>
          {items.map((s, i) => (
            <div className="rp-skill" key={i}>
              <span className="rp-skill-cat">{s.category}:</span> {s.items.join(", ")}
            </div>
          ))}
        </section>
      );
    }
    if (k === "education") {
      const items = doc.education.filter((e) => e.school || e.degree);
      if (!items.length) return null;
      return (
        <section className="rp-sec" key={k}>
          <h2>Education</h2>
          {items.map((e, i) => (
            <div className="rp-entry" key={i}>
              <div className="rp-entry-h">
                <span className="rp-role">{[e.degree, e.field].filter(Boolean).join(", ")}</span>
                <span className="rp-meta">{[e.location, e.date].filter(Boolean).join(" · ")}</span>
              </div>
              {e.school && <div className="rp-school">{e.school}</div>}
            </div>
          ))}
        </section>
      );
    }
    if (k === "projects") {
      const items = doc.projects.filter((pr) => pr.name || pr.bullets.some(Boolean));
      if (!items.length) return null;
      return (
        <section className="rp-sec" key={k}>
          <h2>Projects</h2>
          {items.map((pr, i) => (
            <div className="rp-entry" key={i}>
              <div className="rp-entry-h">
                <span className="rp-role">{pr.name}</span>
                <span className="rp-meta">{pr.url}</span>
              </div>
              <ul>
                {pr.bullets.filter((b) => b.trim()).map((b, j) => <li key={j}>{stripCites(b)}</li>)}
              </ul>
              {pr.tools.length > 0 && <div className="rp-tools">{pr.tools.join(" · ")}</div>}
            </div>
          ))}
        </section>
      );
    }
    return null;
  };

  return (
    <div className="rp" id="resume-print">
      <header className="rp-head">
        <div className="rp-name">{p.name || "Your name"}</div>
        {p.title && <div className="rp-title">{p.title}</div>}
        {contact.length > 0 && <div className="rp-contact">{contact.join("  ·  ")}</div>}
      </header>
      {doc.sectionOrder.map(section)}
    </div>
  );
}
