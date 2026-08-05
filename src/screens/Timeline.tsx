type Entry = { time: string; text: string; chip?: string; kind?: "win" | "collab" | "warn" };
type Day = { label: string; count: number; items: Entry[] };

const DAYS: Day[] = [
  {
    label: "Today — Thursday, 4 Aug",
    count: 4,
    items: [
      { time: "2:10 pm", text: "Reframed the onboarding epic into five user stories", chip: "win", kind: "win" },
      { time: "10:00 am", text: "Sprint planning", chip: "meeting · 2 contributions", kind: "collab" },
      { time: "9:30 am", text: "Roadmap review with nine stakeholders", chip: "needs an outcome", kind: "warn" },
    ],
  },
  {
    label: "Wednesday, 3 Aug",
    count: 5,
    items: [
      { time: "4:40 pm", text: "Cut checkout latency 38% with the retry rework", chip: "win · metric ✓", kind: "win" },
      { time: "1:00 pm", text: "1:1 with Dana", chip: "meeting · action items", kind: "collab" },
      { time: "10:15 am", text: "Reviewed the billing spec", chip: "collaborative", kind: "collab" },
    ],
  },
  {
    label: "Tuesday, 2 Aug",
    count: 3,
    items: [
      { time: "3:20 pm", text: "Shaped the Q3 roadmap draft" },
      { time: "9:45 am", text: "Design review", chip: "meeting", kind: "collab" },
    ],
  },
];

export default function Timeline() {
  return (
    <div className="screen">
      <div className="top">
        <div>
          <h1>Timeline</h1>
          <div className="day">12 captures this week · everything timestamped</div>
        </div>
        <div className="pill trust"><span className="d" />on-device · nothing left this machine</div>
      </div>

      <div className="tl">
        {DAYS.map((day) => (
          <div className="tl-day" key={day.label}>
            <div className="dh">
              <span className="d">{day.label}</span>
              <span className="c">{day.count} captured</span>
            </div>
            <div className="tl-items">
              {day.items.map((it, i) => (
                <div className="tl-i" key={i}>
                  <span className="time">{it.time}</span>
                  <div className="tt">
                    {it.text}
                    {it.chip && <span className={`chip ${it.kind === "collab" ? "collab" : it.kind === "warn" ? "warn" : ""}`}>{it.chip}</span>}
                  </div>
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
