import { Mic } from "../components/icons";

export default function Today() {
  return (
    <div className="screen">
      <div className="top">
        <div>
          <h1>Today</h1>
          <div className="day">Thursday, 4 August · 6h 20m tracked · 4 wins captured</div>
        </div>
        <div className="badges">
          <div className="pill listen"><span className="d" />Listening · Design review</div>
          <div className="pill trust"><span className="d" />on-device · nothing left this machine</div>
        </div>
      </div>

      <div className="sec-label">The day's package</div>
      <div className="pkg">
        <div className="mtg">
          <div className="mtg-h">
            <Mic />
            <div>
              <div className="tt">Sprint planning</div>
              <div className="mm">45 min · 10:00 am</div>
            </div>
            <span className="tag">on-device · notes only</span>
          </div>
          <div className="mtg-grp">
            <div className="lab">Decisions</div>
            <ul>
              <li><span className="b">–</span>Ship export-to-CSV this sprint; defer SSO to next.</li>
              <li><span className="b">–</span>Cut the settings redesign from scope.</li>
            </ul>
          </div>
          <div className="mtg-grp">
            <div className="lab">Action items</div>
            <ul>
              <li><span className="b">–</span>You — write acceptance criteria for export (Fri).</li>
              <li><span className="b">–</span>Dana — spike CSV size limits.</li>
            </ul>
          </div>
          <div className="mtg-grp mine">
            <div className="lab">What you contributed</div>
            <ul>
              <li><span className="b">–</span>Proposed the phased rollout that unblocked the estimate.<span className="save">saved to package</span></li>
              <li><span className="b">–</span>Reframed the SSO debate around the audit deadline.<span className="save">saved to package</span></li>
            </ul>
          </div>
        </div>

        <div className="pkg-item">
          <div className="tick">✓</div>
          <div>
            <div className="t">Reframed the onboarding epic into five user stories</div>
            <div className="m">2:10 pm · from your note<span className="chip">acceptance criteria drafted</span></div>
          </div>
        </div>
        <div className="pkg-item">
          <div className="tick gap">!</div>
          <div>
            <div className="t">Ran the roadmap review with nine stakeholders</div>
            <div className="m">9:30 am<span className="chip warn">needs an outcome</span></div>
          </div>
        </div>
      </div>

      <div className="sec-label">With your senior</div>
      <div className="chat">
        <div className="bub you">Help me turn "let users export their data" into a proper story.</div>
        <div className="bub per">
          <div className="who">Peregrine · senior PM</div>
          As a workspace admin, I want to export all workspace data as a CSV, so that I can satisfy
          an audit without waiting on engineering. Acceptance: export completes under 30s for 100k
          rows; includes deleted-item tombstones; emailed link expires in 24h. Want me to add the
          edge cases a senior would flag?
          <span className="src">drafted with you · saved to today's package</span>
        </div>
      </div>

      <div className="debrief-card">
        <span className="moon">☾</span>
        <div className="txt">
          <b>Tonight's debrief.</b> The roadmap review is missing its outcome — I'll ask a few
          questions and show you where to find the number.
        </div>
      </div>

      <div className="compose">
        <textarea rows={2} placeholder="Tell your senior what you're working on…" />
        <button className="btn icon primary" aria-label="Send">↑</button>
      </div>
    </div>
  );
}
