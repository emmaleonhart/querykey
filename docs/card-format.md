# The Card — P2P broadcast format

> **Status: spec (Round 2, 2026-05-15). Highest-leverage open design
> question.** The card format ossifies fast once people start
> exchanging cards, so it is specced *before* any peer-to-peer code is
> written. **No exchange/transport code this round.** The card layer is
> built *after* the solo PRM (it is a window into a graph you already
> built). Authoritative alongside `CLAUDE.md` / `queue.md`.

## What a card is

A single markdown file each user broadcasts to the network: a curated,
**selective window into the private graph they already built** — not a
separate publishing chore, and not their whole graph.

It carries, fittingly for *QueryKey*, a **key** and a **query**:

- **Key** — what you're *offering* (skills, resources, help, things you
  can connect people to).
- **Query** — what you're *looking for* (help, intros, collaborators,
  things you need).

This makes coordination legible and positive-sum: people can see where
their queries meet others' keys.

## File & example

`card.md` at the vault root. **Git-tracked** (your own history is kept
so you can revert — see asymmetry below).

```markdown
---
id: card:jsmith
handle: github:jsmith          # identity bootstrap (swappable)
updated: 2026-05-15T22:10:00
visibility: public             # only value for now (see open questions)
---

# Emma — card

## Offering (key)
- Rust / embedded DB help (built a graph-vector-time DB)
- Intros into the rationalist/EA Vancouver scene

## Looking for (query)
- Flutter desktop reviewers
- A co-author for a NeurIPS-style writeup

## Notes
Freeform. Best reached via GitHub or Discord.
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

## Open questions (resolve before P2P code)

- **Transport.** What actually moves a card peer-to-peer (a shared
  GitHub org repo as a stepping stone? a relay like Nostr? true P2P)?
  Deliberately unresolved; the *format* must not assume the transport.
- **Private vs. public card.** Planned but explicitly **not now** —
  more complex; revisit after the single public-card model works.
  `visibility:` is in the frontmatter only to reserve the field.
- **Propagation mechanics.** Where the 24h timer lives, how revert-
  before-propagation is detected, what a peer sees mid-delay.
- **Card ↔ graph projection.** Which private nodes a user surfaces into
  the card, and whether that selection is manual or assisted.

None of these block the *format* spec; they block the P2P layer.
