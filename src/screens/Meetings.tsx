import { useEffect, useState } from "react";
import { Mic } from "../components/icons";
import {
  whisperStatus,
  listenStart,
  listenStop,
  captureMeeting,
  listEvents,
  inTauri,
  type WhisperStatus,
  type VaultEvent,
} from "../api";

function payloadText(e: VaultEvent): string {
  return (e.payload as { text?: string })?.text ?? "";
}
function fmtDate(ts: number) {
  return new Date(ts).toLocaleDateString(undefined, { weekday: "short", month: "short", day: "numeric" });
}

export default function Meetings() {
  const [ws, setWs] = useState<WhisperStatus | null>(null);
  const [listening, setListening] = useState(false);
  const [status, setStatus] = useState("");
  const [notes, setNotes] = useState("");
  const [meetings, setMeetings] = useState<VaultEvent[]>([]);

  async function refresh() {
    try {
      setMeetings((await listEvents(200)).filter((e) => e.kind === "meeting"));
    } catch {
      /* ignore */
    }
  }

  useEffect(() => {
    if (!inTauri) return;
    whisperStatus().then(setWs).catch(() => {});
    refresh();
  }, []);

  async function start() {
    setStatus("");
    setNotes("");
    try {
      await listenStart();
      setListening(true);
    } catch (e) {
      setStatus(String(e));
    }
  }

  async function stop() {
    setListening(false);
    setStatus("Transcribing on-device…");
    try {
      const transcript = await listenStop();
      if (!transcript.trim()) {
        setStatus("No speech detected.");
        return;
      }
      setStatus("Writing notes…");
      const n = await captureMeeting(transcript);
      setNotes(n);
      setStatus("Saved — audio discarded, notes kept.");
      refresh();
    } catch (e) {
      setStatus(String(e));
    }
  }

  const ready = ws?.present ?? false;

  return (
    <div className="screen">
      <div className="top">
        <div>
          <h1>Meetings</h1>
          <div className="day">Passive notes · on-device · audio never leaves</div>
        </div>
        {listening && <div className="pill listen"><span className="d" />Listening…</div>}
      </div>

      {!ready ? (
        <div className="pkg-empty">
          To listen in meetings, point Peregrine at a local Whisper model — set the <b>Whisper model path</b> in Settings
          (a <span className="mono">ggml-base.en.bin</span> from whisper.cpp works). Transcription then runs entirely on
          your machine; no audio is ever uploaded.
        </div>
      ) : (
        <>
          <div className="mtg" style={{ display: "flex", alignItems: "center", gap: 14 }}>
            <Mic />
            <div style={{ flex: 1 }}>
              <div className="mtg-h" style={{ display: "block" }}>
                <div className="tt">{listening ? "Listening — it won't interrupt" : "Ready to listen"}</div>
                <div className="mm">{status || "Starts capturing your mic; on-device Whisper transcribes it."}</div>
              </div>
            </div>
            {listening ? (
              <button className="btn" onClick={stop}>Stop &amp; save notes</button>
            ) : (
              <button className="btn primary" onClick={start}>Start listening</button>
            )}
          </div>

          {notes && (
            <>
              <div className="sec-label">Latest notes</div>
              <div className="mtg"><div style={{ whiteSpace: "pre-wrap", fontSize: 13, lineHeight: 1.5 }}>{notes}</div></div>
            </>
          )}
        </>
      )}

      {meetings.length > 0 && (
        <>
          <div className="sec-label">Past meetings</div>
          <div className="pkg">
            {meetings.map((e) => (
              <div className="mtg" key={e.id}>
                <div className="mtg-h">
                  <Mic />
                  <div className="mm">{fmtDate(e.ts_ms)}</div>
                  <span className="tag">on-device · notes only</span>
                </div>
                <div style={{ whiteSpace: "pre-wrap", fontSize: 12.5, lineHeight: 1.5, marginTop: 8 }}>{payloadText(e)}</div>
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
