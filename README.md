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
