export default function Placeholder({
  title,
  day,
  intro,
  fields,
  phase,
}: {
  title: string;
  day: string;
  intro: string;
  fields: string[];
  phase: string;
}) {
  return (
    <div className="screen">
      <div className="top">
        <div>
          <h1>{title}</h1>
          <div className="day">{day}</div>
        </div>
      </div>
      <div className="empty">
        <h2>{title} — coming next</h2>
        <p>{intro}</p>
        <ul>
          {fields.map((f) => (
            <li key={f}>{f}</li>
          ))}
        </ul>
        <span className="soon">{phase}</span>
      </div>
    </div>
  );
}
