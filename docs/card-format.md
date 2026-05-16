# The Card — P2P broadcast format

> **Status: format + local layer IMPLEMENTED (Round 7, 2026-05-16).**
> The PRM/vault landed (Rounds 5–6), so per the adoption sequencing
> the card layer was built next. **What is built:** the card *format*
> (`src/card`, `card.md` at the vault root, the stable `## Offering` /
> `## Looking for` heading contract, lossless round-trip), the
> swappable *identity* abstraction (`src/identity`, GitHub bootstrap),
> the local *24h propagation safety valve* (working↔published with
> revert-before-propagation), the privacy `.gitignore` *asymmetry*
> (your card tracked; peers' ignored), and a read-only *peers* path +
> the `/api/card|identity|peers` endpoints. **What is deliberately
> NOT built:** the **transport** that actually moves a card between
> peers — still the highest open question; the format does not assume
> it and there is no exchange/relay code. Authoritative alongside
> `CLAUDE.md` / `queue.md`. Spec below matches the build; deviations
> are under "Implementation notes".

## What a card is

A single markdown file each user broadcasts to the network: a curated,
**selective window into the private graph they already built** — not a
separate publishing chore, and not their whole graph.

It carries, fittingly for *QueryKey*, a **key** and a **query**:

- **Key** — what you're *offering* (skills, resources, help, things you
  can connect people to).
- **Query** — what you're *looking for* (help, intros, collaborators,
  things you need).

This makes coordination legible and positive-sum: other people's agents
attend over your key/query when figuring out who is worth connecting to.

**V is not on the card.** Completing the Q/K/V metaphor: the *value* is
not a stored or published field — it's whatever emerges in the real
world when people actually connect and do things together. The system
facilitates the attention (Q against K) and then gets out of the way;
it deliberately does not try to measure, score, or gamify the output.
That epistemic humility is a feature, especially for the rationalist
audience.

### Card vs. profile (document this — it prevents scope creep)

The card is a **signal, not a profile**. Keep it lean: query, key, and
at most a short bio or a **link out to your personal website**. The
website (which people already curate) is the substance / source of
truth for *who you are*; the card is just the hook that points to it.
People will constantly want to grow the card — the standing answer is
*"the card is your query and your key; everything else is your
website."* A small, stable card also keeps the P2P payload tiny and
easy to sync.

### Agent-drafted, human-approved

Most people — especially younger people — are bad at articulating their
own value. So the card is **drafted by your local agent**, which has
been building your PRM by observing your conversations, commitments,
and what energizes you; it is in a better position to write a first
draft of your `key` (and notice patterns for your `query`) than you
are. You **review, curate, and approve** before it goes out. This is a
more honest representation than a self-reported form. How the agent
drafts it is governed by `agents.md` (see below / `CLAUDE.md`).

## File & example

`card.md` at the vault root. **Git-tracked** (your own history is kept
so you can revert — see asymmetry below).

```markdown
---
id: card:jsmith
handle: github:jsmith          # identity bootstrap (swappable)
website: https://emmaleonhart.com   # the substance lives here
updated: 2026-05-15T22:10:00
visibility: public             # only value for now (see open questions)
---

# Emma — card

> Short bio: builds local-first tools; rationalist-adjacent. (One line —
> the real depth is the website above.)

## Offering (key)
- Rust / embedded DB help (built a graph-vector-time DB)
- Intros into the rationalist/EA Vancouver scene

## Looking for (query)
- Flutter desktop reviewers
- A co-author for a NeurIPS-style writeup
```

The body is intentionally human-first markdown; the `Offering` /
`Looking for` headings are the machine-parseable contract. Keep the
heading names stable — parsers key off them.

## The privacy model (the actual differentiator)

This inverts how social networks normally work. Normally the network
owns your history and you fight to delete things. Here:

1. **Asymmetric git-tracking.**
   - *Your* card is tracked in *your* repo (you need history for undo).
   - *Other people's* cards live in `peers/<handle>/card.md` and are
     **git-ignored on your machine**. You can use the information in
     the moment; you do **not** build a surveillance archive of how
     someone's card changed over time.
2. **24-hour propagation delay.** A card edit does not go out
   immediately. A drunk 11pm mistake can be corrected by morning and
   *nobody ever saw it*. If you catch it before propagation, an
   immediate revert needs no delay.
3. **Absence of history is the default.** Persistence of any past
   state requires a deliberate observer capturing it at exactly the
   right moment. The guarantee is **soft and social, not
   cryptographic** — appropriate for a community that does not assume
   bad actors. State this honestly to users; don't oversell it.

## Identity & discovery

- **GitHub bootstraps both.** Username = handle; "follow on GitHub" =
  the discovery/subscription mechanism for whose cards you pull.
- Treat identity as a **thin abstraction**: *"a user is a canonical
  handle that currently resolves via GitHub."* Do not bake GitHub into
  call sites — it should be swappable for DIDs/Nostr later without a
  rewrite.
- **Pure peer-to-peer.** No central server, no global source of truth.
  Cards move directly between peers.

## Implementation notes (Round 7 — as built)

- **Format** lives in `server/src/card/`. `Card` =
  handle/name/website/bio/offering/looking_for/updated/visibility.
  `offering` is the **key**, `looking_for` the **query**; there is no
  `value` field by design (V is the real-world outcome — never stored
  or scored). `render`/`parse` round-trip losslessly and are
  idempotent (unit-tested); parsing is tolerant of the `(key)`/
  `(query)` heading suffixes and extra prose, but the `## Offering` /
  `## Looking for` heading names are the contract.
- **Identity** is `server/src/identity/` — a `CanonicalHandle`
  (`scheme:localpart`) + an `IdentityProvider` trait. `GitHubIdentity`
  normalizes every input form (`jsmith`, `@jsmith`,
  `https://github.com/jsmith`, …) to one handle. `default_provider()`
  is the **only** site that names GitHub — swap there for DID/Nostr.
  *Discovery* (whose cards you pull) is part of the transport question
  and is not implemented (no network calls).
- **Propagation mechanics (resolved, local side):** `card.md` is the
  working/tracked file; `.querykey/card.pending.md` + `card.eligible_at`
  is the staged edit; `.querykey/card.published.md` is the frozen
  snapshot a transport *would* broadcast. An edit stages pending with
  a 24h window and never touches the published snapshot; promotion is
  lazy (computed on every card read); revert-before-propagation drops
  the pending edit and restores `card.md` from the published snapshot
  immediately. `.querykey/` is git-ignored, so this state never enters
  history.
- **Asymmetry enforced:** `Vault::open` writes a `.gitignore` into the
  vault root ignoring `/peers/` and `/.querykey/` but **not**
  `card.md` (idempotent; appends rather than clobbering an existing
  one). Peer cards are read-only from `peers/<fs-safe-slug>/card.md`;
  `:` in a handle never hits the filesystem.
- **API:** `GET|PUT /api/card`, `GET /api/card/published`,
  `POST /api/card/revert`, `GET /api/identity`, `GET /api/peers`,
  `GET /api/peers/:handle/card`.

## Open questions (still open — block the P2P transport, not the format)

- **Transport.** What actually moves a card peer-to-peer (a shared
  GitHub org repo as a stepping stone? a relay like Nostr? true P2P)?
  Still deliberately unresolved; the format + local layer do not
  assume it. This is now *the* gating question for the P2P layer.
- **Discovery.** "Follow on GitHub" → whose `peers/` you populate.
  Tied to transport (needs network); intentionally unbuilt.
- **Private vs. public card.** Planned but explicitly **not now** —
  more complex; revisit after the single public-card model works.
  `visibility:` is in the frontmatter only to reserve the field.
- **Card ↔ graph projection.** Which private nodes a user surfaces into
  the card (the agent-drafted `key`/`query`), and whether that
  selection is manual or `agents.md`-assisted. The vault/PRM it would
  draw from exists (Rounds 5–6); the drafting heuristic does not yet.
