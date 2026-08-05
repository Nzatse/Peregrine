<div align="center">

# 🦅 Peregrine

### A local-first AI coworker that quietly remembers your wins.

*Part of the **Aerie** suite — alongside [Kestrel](https://github.com/Nzatse/Kestrel), the interview copilot.*

**Kestrel watches the moment. Peregrine tracks the journey.**

</div>

---

## The problem

Every day you accomplish real things at work — you ship the fix, lead the meeting, unblock the team, rewrite the thing that was slow. And almost all of it evaporates. By the time you sit down to update your résumé, write a self-review, or make the case for a promotion, you're reconstructing months of work from memory — and selling yourself short.

It isn't that people lack accomplishments. It's a **translation problem**:

> *What you did:* "Spent three weeks debugging checkout and rewrote the payment retry logic."
>
> *What belongs on a résumé:* "Reduced failed transactions 34% by re-architecting payment retry logic, recovering ~$1.2M in annual revenue."

People can't make that jump because (a) the details are gone by the time they need them, and (b) they think in *tasks*, not *outcomes*. Peregrine closes both gaps — continuously, while the work is still fresh.

## What Peregrine is

Peregrine is a **local AI coworker** you download and run on your own machine — no admin rights, no cloud account, no work data leaving your computer. It:

- 🤝 **Works alongside you** — reviews your projects, proposes where to take them, and brainstorms like a genuinely good colleague.
- 🧠 **Remembers what you accomplish** — because it was there when you did it. This becomes your private **career memory**.
- ✍️ **Renders that memory into anything** — résumé bullets, a LinkedIn summary, a self-review, a promotion packet, a weekly brag doc — tailored to the platform and the moment.
- 🔌 **Connects outward on your command** — read your GitHub/GitLab, LinkedIn, or Indeed with one click, and Peregrine tells you *exactly what belongs in which section.* You review and publish; Peregrine never posts for you.
- 📄 **Builds on your existing résumé** — it already knows your whole trajectory, so it improves what you have instead of starting from a blank page.

The coworker is the product you'd open anyway. The career artifacts are the proof it was paying attention.

## Why it's built this way — trust is the architecture

Peregrine is meant for **every professional** — engineers, nurses, lawyers, teachers, PMs — which means it handles sensitive work. Trust here isn't a promise; it's something you can **verify**.

### The core principle: no surprises

> Everything stays on your computer. The only thing that ever leaves is the prompt to the AI model *you* personally chose — and you can watch it happen, and turn it off anytime.

### How that's enforced

- **🏠 Local-first.** The app runs entirely in user space. Your career memory is a local, encrypted store you own — export it or delete it wholesale, anytime. No lock-in.
- **🔑 Bring your own model.** Plug in your own API key (Claude, OpenAI, or others — US-jurisdiction providers), or run a fully **local model** for true zero-export. You control the data relationship.
- **🚦 Asymmetric egress ("data diode").** Peregrine can pull information *in*, but never pushes results *out* on its own. An inspectable **egress allowlist** limits outbound traffic to exactly your chosen model endpoint and the read-only sources you connected — nothing else.
- **📡 Zero telemetry.** No analytics, no crash-phone-home, no hidden connections. An in-app **Activity panel** shows, in plain English, everything that left your machine and when.
- **✋ Human-in-the-loop.** Peregrine *drafts*; you *ship*. Nothing is ever published, posted, or sent without you seeing the exact text and clicking.
- **🔍 Provenance, never fabrication.** Every generated claim links back to the evidence that produced it. Peregrine **never invents a metric** — it asks you for the number. "You said this" and "we inferred this" stay visually distinct. (On a résumé, a hallucinated number can cost someone a job.)

### The trust dial

You choose your egress posture; the same app serves all three:

| Mode | Model | Where your work data goes | For |
|---|---|---|---|
| **Airtight** | Local model (on-device) | Nowhere. Truly zero export. | Regulated / privileged data |
| **Trusted cloud** | Claude / OpenAI zero-retention endpoint | To that one provider — not trained on, not retained | Most professionals |
| **Standard cloud** | Consumer API key | To that provider under normal terms | Low-sensitivity work |

We will never claim "nothing leaves" while routing prompts to a cloud model. Airtight mode means it literally; cloud modes mean *one explicitly-trusted endpoint, honestly disclosed* — with the wire open for you to check.

## How it works

```
┌─────────────┐     ┌──────────────────────┐     ┌─────────────┐
│  1. CAPTURE │ ──▶ │ 2. ACCOMPLISHMENT     │ ──▶ │ 3. RENDER   │
│             │     │    ENGINE             │     │             │
│ Projects,   │     │ Cluster → narrativize │     │ Résumé,     │
│ git/PRs,    │     │ → quantify → score.   │     │ LinkedIn,   │
│ tickets,    │     │ Turns raw work into   │     │ self-review,│
│ calendar,   │     │ outcome-based wins    │     │ promo doc,  │
│ your notes  │     │ and asks for the      │     │ brag doc    │
│             │     │ missing metric.       │     │             │
└─────────────┘     └──────────────────────┘     └─────────────┘
     (local)              (the moat)                (on click)
```

1. **Capture** — Peregrine learns what you did from the projects and sources you point it at. Scoped and consented: it sees only what you grant, never roams your disk.
2. **Accomplishment engine** — the hard part, and the heart of Peregrine. It clusters scattered signals (12 commits + a PR + a thread) into a single *accomplishment*, frames it as an outcome, and nudges you for the metric while you still remember it.
3. **Render** — one career memory, many outputs. Each artifact is a different view over the same store, tailored to where it's going.

## A senior who guides you — role-aware mentorship

Peregrine isn't a generic chatbot — and it isn't just for engineers. **Whatever your profession, Peregrine plays the senior version of your role**: the colleague who's seen it all and guides you. A senior PM for a product manager, a charge nurse for a nurse, a senior partner for a lawyer, a department head for a teacher, a creative director for a designer — it adapts its mentorship to *your* field through profession-specific **role packs**.

Take an **IT engineer** as just one example — there, that senior wears several hats on demand:

- 🧩 **Product owner** — turns your goal into proper **user stories** (*"As a…, I want…, so that…"*) with acceptance criteria, breaks down epics, grooms and prioritizes a backlog.
- 🛠️ **Tech lead / staff engineer** — weighs trade-offs, reviews your plan or code, flags what a senior would catch, points you to best practices.
- 🎓 **Mentor** — when you don't know something, it explains it *and* shows how a senior reasons about it, so you level up instead of just getting unblocked.

Every profession gets its own set of hats: a nurse's senior helps with care plans and charting, a PM's helps shape the roadmap and write specs, a marketer's sharpens the campaign brief, a researcher's pressure-tests the method. The engine is the same — the expertise is *yours*.

> **This is what earns the seat.** You open Peregrine for the senior guidance — and while it helps you work, it quietly captures that very work as your career memory. Mentorship and memory-building are the same motion: everything it helps you produce (a set of user stories, an architecture decision) is filed as an accomplishment automatically. It guides, it never bluffs — bound by the same *never fabricate* rule, it flags uncertainty instead of inventing an answer.

## Your day with Peregrine

Peregrine is built around a daily rhythm:

- ☀️ **Through the day** — you narrate and write about your work as it happens; Peregrine helps in the moment (guidance, user stories, review, direction) and assembles **the day's package** — a live, structured record of what you did, including the *partial and collaborative* bits ("I unblocked someone," "I reviewed the design") that usually evaporate. It keeps track of *when* things happened.
- 🌙 **At night — the debrief.** Peregrine reviews the day's package, finds the **unfinished stories** (a win with no metric, a task with no outcome), and **interviews you** to fill them while they're fresh. If you can't answer — *"I don't know the impact number"* — it tells you **how to find out** (*"check the linked ticket," "ask Maria for the before/after," "that dashboard has it"*). The completed package is filed into your memory.
- 📈 **Over time — it improves you.** The debrief shows you a stronger version of your own work, names the skill you just demonstrated, and tracks your growth — not just your wins.

## Your career memory is a file you own

Your work history lives in a single **portable, encrypted `.peregrine` vault** — a thing you hold, not a database locked to one app or machine:

- 📼 **Event-sourced & append-only.** Every capture is an immutable event. History is additive, never overwritten — nothing gets silently lost.
- 🔐 **Encrypted with your passphrase.** It travels across machines and decrypts only for you. There's **no backdoor** — the price of it being truly yours.
- 🔄 **Work ↔ home, no work lost.** Because it's an append-only log, syncing two machines is just the **union of their events** — lossless and conflict-free, even if you capture on both the same day. Whatever syncs is encrypted; the transport (your own cloud folder, or manual carry) never sees readable data.
- 🧳 **Versioned.** A newer Peregrine can always open and upgrade an older vault.

## How Peregrine and Kestrel share your history

Peregrine and [Kestrel](https://github.com/Nzatse/Kestrel) read and write the **same career vault** — that shared memory is the connective tissue of the Aerie suite:

- 🏦 **Peregrine fills the bank** — all year, every debrief, it captures and quantifies what you did.
- 🎤 **Kestrel makes the withdrawal** — in a live interview, it draws on that history to *put words where you freeze*: asked about a project, it already knows what you did, quantified, and helps you say it.

> Peregrine spends the year building your story; Kestrel delivers it the moment the interviewer asks. The better Peregrine does its job, the less you ever freeze.

## The meeting listener — quiet notes that matter

So much of your real contribution happens in meetings — and it's the first thing to evaporate. Peregrine can **sit quietly in the background and listen, taking notes without ever intervening.** It doesn't interrupt, prompt, or speak (that's Kestrel's job, in interviews) — it just listens and writes.

From a meeting it produces a clean summary, the **decisions**, the **action items**, and — most importantly for you — **the parts *you* contributed**, flagged straight into the day's package. That's how "the bits I participated in" finally stop disappearing.

Built to the same trust rules:

- 🖥️ **On-device only.** Audio is transcribed locally — **no audio and no transcript ever leaves your machine.**
- 🗑️ **Transcribe-and-discard.** By default Peregrine keeps the *notes*, not the recording — there's no audio file left behind. *(You can opt to retain audio.)*
- 👁️ **Never covert.** A clear "listening" indicator is visible the whole time; you start it deliberately; it never records on its own.
- 🤫 **Passive.** It only listens and notes — no interjections, no live prompts.
- ⚖️ **Consent is yours.** Peregrine can remind you, but telling participants and following your workplace's policy is on you.

*Capturing meeting audio needs the OS microphone/audio permission (a one-time grant you approve — not admin rights), and on-device transcription uses an optional model download, so the base app stays light. It reuses the same audio + transcription foundation as Kestrel.*

## Make it yours — appearance modes

All three of Peregrine's design directions ship built in — pick the one that fits how you work (each also follows your system light/dark):

- 📓 **Fieldbook** — cool paper and ink, serif headings. Calm, senior, human.
- 🎛️ **Instrument** — ink-slate dark, precise, trust status always in view. A power-tool feel.
- ☀️ **Daylight** — soft and approachable, welcoming for every profession.

## The interface

Peregrine's UI is built on design tokens, so the three looks (see [appearance modes](#make-it-yours--appearance-modes)) are **one interface reskinned, not three codebases**. A few principles hold across all of them:

- 🛡️ **Trust is always on screen** — the egress / airtight status and the meeting "listening" indicator live top-right, never hidden.
- 🎨 **Semantic state colors** — green = captured, amber = needs an outcome or metric, blue = *your* contribution — kept separate from each mode's accent.
- 📄 **Summary before detail**, every draft carries a provenance line, and the human always ships.

The core screens:

- **Today** — the day's package (captured wins, meeting notes, the collaborative bits), your senior-mentor thread, and tonight's debrief prompt.
- **Timeline** — every capture laid out by day and time. Because the vault is event-sourced, *when each thing happened* is always answerable — for you, and for Peregrine at the nightly debrief.
- **Settings** — bring-your-own-model (endpoint · model · keychain-stored key · connection test), the trust dial, appearance mode, your role, the meeting listener, vault & sync, and privacy.

> **Interactive prototype** — [view the three modes, the meeting listener, and all screens →](https://claude.ai/code/artifact/d2e0a439-037e-455c-b52e-af5e841d7f6d)

## Status

🚧 **Early / pre-alpha.** This repository currently defines the vision and architecture. The brainstorm is done; the build is beginning. Follow along.

**Roadmap (high level):**
- [ ] The coworker core — review a project, propose a trajectory, brainstorm (earns its seat first)
- [ ] Local career-memory store (encrypted, user-owned, exportable)
- [ ] Bring-your-own-model layer + the trust dial
- [ ] Egress allowlist + Activity panel
- [ ] Accomplishment engine (cluster → quantify → score)
- [ ] Renderers: résumé, LinkedIn, self-review
- [ ] One-click read connectors (GitHub / GitLab, then LinkedIn / Indeed)
- [ ] Senior mentor — role-aware guidance (product-owner user stories, tech-lead review) starting with IT engineer
- [ ] The daily loop — assemble "the day's package," track time, catch partial/collaborative work
- [ ] The nightly debrief — gap detection + interview + "how to find out" coaching
- [ ] Portable event-sourced encrypted `.peregrine` vault (append-only, passphrase, versioned)
- [ ] Work ↔ home sync — union-of-events over your own encrypted cloud folder
- [ ] Shared vault with Kestrel — your history powers live interview help
- [ ] Meeting listener — passive, on-device transcription; notes, decisions, action items, and *your* contributions into the day's package
- [ ] Appearance modes — Fieldbook / Instrument / Daylight, plus system light/dark
- [ ] Timeline view — every capture laid out by day and time (the event-sourced vault, surfaced)

## The Aerie suite

Peregrine is one falcon in the **Aerie** — a family of local-first, privacy-respecting professional tools.

| Tool | Role |
|---|---|
| **[Kestrel](https://github.com/Nzatse/Kestrel)** | Interview copilot — real-time help *in* the interview |
| **Peregrine** *(this repo)* | Work coworker — tracks the journey and builds the record that gets you *to* the interview |

The two are separate today and will converge under the Aerie roof over time.

## License

Licensed under the **[GNU AGPL-3.0](LICENSE)**. You're free to read, run, modify, and share Peregrine — but anyone who offers it as a network service must share their changes back under the same license. That keeps the project **source-available and auditable** (essential for a tool you're trusting with your work) while preventing it from being taken closed.

This repository is **private** during early development; a public release is planned so the community can help build and audit it.

---

<div align="center">

*Built with care for people who do great work and deserve to be able to prove it.*

</div>
