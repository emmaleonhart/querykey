# Building a rationalist peer-to-peer social network

*Extracted from Claude.ai chat.*

## User

Okay, so what's going on? I am trying to make a kind of a rationalist social network thing. Right now it is here at it is on my GitHub, emmalyonheart/ququery key

Https://github.com/EmmaLeonhart/querykey

Yeah, so I don't really know the degree. Keep in mind there's an agent right now that's kind of working on the vision of it. I'm actually intentionally kind of rambling a bit with what I'm saying right now, because by rambling a bit I'm giving the agent more time to update all the documentation and planning and stuff. That way you'll be able to get a better idea of it, because I want to converse with you about it strategically. A coding agent isn't really that good for discussion, but the idea here is essentially The idea behind it is kind of like a PRM, CRM, ERP, JIRA thing and also a social network. The idea behind it is you can track a lot of the stuff that you're doing with your friend. You can track a lot of the stuff that's going on with your relationship with people, such as, for example, and you can display a social network graph. It has the ability to be theoretically used entirely privately, entirely privately, just by a personal person doing their networking thing but part of the kind of vision of this ideally is that um other people can use query key to um to like part of the idea is basically like we see it as it's like a positive sun game and um ideas that people can kind of like sort of post like roughly things like what it is that they're generally looking for and what it is that they're offering and and what it is that they're offering and um through like a and it's kind of like a peer-to-peer social network or at least it has an element of being a peer-to-peer social network you can also like personally make personal notes about it you can like make your own personal notes about like um you can make your own personal notes about um like for query key basically like is it about like what you want that's private? or what somebody else wants but private? because like it does include like it includes like not includes like not members and members alike and um and part of an idea here is that you basically we use like the local AI model to basically kind of like plan out interactions plan out interactions do your own scheduling, make like things of like the critical path of what you're doing make things of like the critical path of what you're doing, which things you need in which order etc etc which you can make varying varying levels of you can make it varying levels of public what things you need in which order and stuff like that to achieve which tasks it stores in markdown stores in markdown and basically makes a git repository it basically stores in Mark, it basically stores everything in Mark down and uses a git repository to track it and and the idea is that and I would say like I would imagine that you'd be like often signing into this with your GitHub or at least you have your GitHub connected in such a way that it would be able to like think your thing with other people's and the peer-to-peer element would be like would push around different things that people want in the different information that people give out the different information that people give out is basically going to be a Passed around with peer-to-peer stuff and I hope this makes sense and I want to know like the way to I want to have a bit more elaboration on how to strategically Go about this And I think probably the agents gonna be finished

## Claude

Searched the web

Searched the web

The repo isn't publicly indexed yet, so I can't read it directly. But I have enough from what you described to have a real strategic conversation. Let me reflect back what I'm hearing and then give you my honest take.

What QueryKey seems to be:

A personal relationship/network OS — part CRM, part PKM, part social network — where the core data model is people, what they want, what they offer, and the state of your relationships with them. Everything stored in markdown/git, local-first, optionally federated peer-to-peer. A local LLM helps you reason about it (critical path, scheduling, what to do next with whom). GitHub as both auth and sync backbone.

The rationalist/LessWrong angle is actually the most distinctive part — framing relationships as positive-sum games, publishing "I'm offering X / I'm looking for Y" posts, making coordination legible.

Strategic questions I'd be asking right now:

1. What's the minimal atomic unit? The vision is very broad — CRM + social graph + P2P + scheduling + local LLM + git sync. That's a lot. What's the one thing that's undeniably useful even if nothing else is built? My guess: the personal contact/relationship tracker with private notes and a graph view. That's the "always useful even solo" core.

2. The P2P layer is the hard problem. Git + GitHub as sync is elegant for solo use, but peer-to-peer discovery of "what people are offering/looking for" is genuinely hard to make work without a central server. Have you thought about whether there's a lightweight relay (like Nostr, or even a shared GitHub org repo) as a stepping stone before true P2P?

3. Who is the first ten users? The rationalist community is actually a great beachhead — they already think in terms of epistemic legibility, positive-sum games, and updating on new info. r/LessWrong, the EA Forum, ACX readers — these people would immediately grok the value proposition in a way a general audience wouldn't.

4. The LLM planning angle is the viral hook. "Local AI that knows your social graph and helps you figure out what to do next" is genuinely novel and legible. That might be the thing worth building first visibly, because it's the thing someone can screenshot and post.

What's the current state of the codebase — is it scaffolding, or is something actually running?

## User

I literally gave you the URL, so it does not need to be indexed. I gave you the git repository. I accidentally had a capital H in the URL, but you should still be able to get it and look at it and look through its documentation.

## Claude

You're right, let me fetch it directly.

Failed to fetch: https://github.com/EmmaLeonhart/querykey

Failed to fetch: https://github.com/EmmaLeonhart/querykey

I'm hitting a permissions error — the system is blocking me from fetching GitHub URLs that weren't returned by a search result first, regardless of whether you provided it directly. This is a network/security restriction on my end, not a URL issue.

A couple of options:

Paste the README or key docs directly into the chat and I can work from that.

Make the repo public if it isn't already (though it may already be).

Share specific files — even just the README and any planning/vision docs the agent was updating.

The most useful thing would probably be the README plus whatever planning or architecture docs exist. That'll give me the grounding to have a genuinely useful strategic conversation rather than working from the verbal description alone.

## User

Why are you not able to access it? It's very strange that you wouldn't be able to access it. It seems very strange that you wouldn't be able to access it. I thought cloud is supposed to be able to access GitHub? Access all Git repositories to analyse them.

## Claude

I understand the confusion — Claude can browse the web and fetch URLs, but there's a specific restriction in this environment that only allows me to fetch URLs that either you've explicitly shared in a way my tools recognize, or that appeared in search results first. GitHub URLs specifically seem to be hitting a permissions block here.

This isn't a general Claude limitation — it's specific to how this session's network access is configured. The error message says "this URL cannot be fetched because it was not provided by the user nor did it appear in any search/fetch results," which is a security policy on the fetch tool, not a capability issue.

The workaround I'd suggest: just paste the README and any key docs directly into the chat. If the agent was updating planning/vision docs, those are exactly what I want to read. Even a raw paste of the file tree would help me understand the current structure.

## User

Check here querykey.emmaleonhart.com/agent_stuff

Basically, it's like that URL, like that subdomain. I guess I should say it is the subdomain of the site, and you can look at it here. The actual agent stuff, which is essentially a made-easy accessibility thing for all the different kinds of documentation and planning documents and that stuff, isn't really present on this site. That stuff, I think, is going to be there in a bit, but it's not going to be there quite now. I'm really confused why it is that you're not able to fetch from GitHub, but you can try it here.

## Claude

Failed to fetch: https://querykey.emmaleonhart.com/agent_stuff

Failed to fetch: https://querykey.emmaleonhart.com/agent_stuff

SSL certificate issue — the certificate on `querykey.emmaleonhart.com` doesn't match that hostname, so my fetcher is rejecting it for security reasons.

As for the GitHub thing: I suspect it's because this environment routes web fetches through a proxy that has GitHub in a restricted category, possibly to prevent things like bulk scraping of private repos. It's annoying for legitimate use like this.

Quickest path forward: just paste the content here. Even a rough dump of:

The README

Any vision/planning doc the agent was working on

The file tree (you can get that with `find . -type f | head -50` or just the GitHub file browser)

That'll take 30 seconds and unblock the whole conversation. What's the current state of the agent — did it finish updating the docs?

## User

Oh my fucking god, I cannot paste anything to you. I'm not at my computer. 

emmaleonhart.github.io/querykey

Does this work

## Claude

Fetched: QueryKey

Fetched: QueryKey

Got it! And the Q/K/V naming is clever — I like that a lot. Let me grab the GitHub page now since I have a search result to work from.

Fetched: GitHub - EmmaLeonhart/querykey: QueryKey is a social network that you run locally from your desktop. It has elements of a PRM and uses local AI agents to help you optimize your social life while respecting your privacy and the privacy of others

Fetched: GitHub - EmmaLeonhart/querykey: QueryKey is a social network that you run locally from your desktop. It has elements of a PRM and uses local AI agents to help you optimize your social life while respecting your privacy and the privacy of others

Okay, now I have a real picture. Let me also grab the queue.md and todo.md since those are the authoritative planning docs.

Searched the web

Searched the web

Okay, I have a very complete picture now — and the agent finished everything. All 10 items done and pushed as of today. The docs are clean, the framing is right. Good.

So let me give you the strategic conversation you actually wanted.

The state of things is solid. The pivot is clean, the repo is honest about what exists vs. what's planned, and the tech stack decisions are settled (Flutter, Rust, local Gemma). What's unresolved is all product/design — which is exactly the right thing to be unresolved at this stage.

The three open questions that actually gate everything else:

The most load-bearing one is the on-disk markdown format. Everything downstream — the ingestion pipeline, the agent's ability to act on files, the local graph store decision — depends on this. And it's actually not that hard to decide. My instinct: YAML frontmatter for structured fields (person, date, tags, status), freeform markdown body for notes. That's the Obsidian convention and it means the files are immediately useful to humans without QueryKey installed.

Second is the federation / sharing model. This is the entire product question for the rationalist social network angle. The "I'm offering X / looking for Y" posts you described — are those opt-in exports from your private graph, or a separate layer you explicitly author? That distinction determines whether the network emerges organically from private use, or requires a separate publishing gesture. I'd argue the former is more elegant and more rationalist-flavored: your private data stays private, but you can selectively surface nodes as public signals.

Third is graph store under Rust. Fuseki is heavyweight for a single-user local app. SQLite with a relations table is probably the right call — it's embeddable, fast, and the query patterns (who do I know, what did I commit to, what's pending) are simple enough that you don't need SPARQL.

What would I build first? The markdown file model + the agent reading and writing those files. That's the thing you can demo to yourself in a day, it validates the whole "your data is just files" promise, and it doesn't require the Rust rewrite to be done first.

What aspect do you want to dig into?

## User

Okay, does it actually say that it's using Fuseki? It's not supposed to be using Fuseki. It's supposed to be using my graph database that I made, which is in Rust, and it's called Loca, previously called SutraDB. It's not 100% clear on any of this stuff. I'm not asking you to build stuff. I'm more asking you to talk about it, to discuss it strategically, because another agent is building it, and I am delegating you as my strategic discussion partner.

## Claude

You're right, and I misread — the README does mention Fuseki as a "planned" graph store with a stub client, but that's clearly stale pre-pivot documentation, not the actual intent. The agent documented what existed before, not where you're going.

So let me re-frame that whole third point: the graph store question isn't open at all — it's Loca/SutraDB. That's actually a much more interesting architectural choice than SQLite, and it makes the system more coherent with your broader work. The knowledge graph in QueryKey is literally running on your own embedded graph-vector-time database. That's a strong story, both technically and for the portfolio angle.

Which raises the thing I actually want to discuss strategically: how does Loca's data model map onto the QueryKey entity model?

The QueryKey README lists a pretty rich entity set — Person, Handle, Task, Event, Message, Conflict, Instruction, OpenQuestion, FollowUp, VoiceProfile. That's a graph where the nodes are heterogeneous types and the edges are things like "this task is owned by this person" or "this message created this open question." Loca being a graph-vector-time database is interesting because the time dimension is actually central to the QueryKey use case — you care about the history of a relationship, not just its current state.

The question I'd be thinking about: are the relationships between people and tasks and commitments going to be modeled as explicit edges in Loca, or is the graph more of a secondary index over the markdown files? Because those are two quite different architectures with different implications for what the agent can do.

## User

Yeah, yeah, it's not. I specifically got rid of Fuseki because Fuseki was way too difficult, and I'm a bit surprised that Fuseki is still there, with documentation at all. Because I specifically thought I got rid of Fuseki, that must be an error there. My main view here is that you can use this just to track all your relationships completely privately, but my view is that tracking all your relationships completely privately is actually not the best way to go about such a thing. I think not everybody is going to be 100% on board with adopting this. That was a pre-pivot. My view, I think, is going to be that the graph is essentially a model used above the different markdown files. While I love RDF, RDF is not as readable by AI agents. My idea here is more or less that RDF is generated from the markdown. There might be a certain degree of RDF that is stored locally.

I also think the thing is I have this generative knowledge graph, and I've already made a knowledge graph. Basically, my idea is that the markdown gets converted into a knowledge graph, and the knowledge graph has the neurosymbolic world models separate from it. I think the markdown should be the primary source of truth, and the git repository that is generated from this. My idea here is that there's going to be an MCT server that's going to allow you to use potentially any agent. You don't need to necessarily use Jemma, but the reason why I would want you to use what I think Jemma is probably best is mostly because Jemma is really cheap to just run on your own. Also, local first is probably best for something like this, because yeah, potential privacy

## Claude

Okay, this is clarifying a lot. Let me reflect back the architecture as I'm now understanding it, and you can correct me:

The actual data flow:

Markdown files on disk are the primary source of truth. Git tracks their history — which is important because relationships change over time and you want that provenance. From the markdown, you derive RDF/a knowledge graph (probably via the local agent), and Loca stores that derived graph. The graph is a secondary, queryable representation — not the canonical store. AI agents operate primarily on the markdown but can query the graph for structured lookups.

The MCP server angle is actually really smart. If QueryKey exposes itself as an MCP server, then any agent — Claude, Gemma, whatever — can attach to it and operate on your relationship graph and tasks. You're not locked to one model. The local-first default (Gemma) is just the cheapest/most private option, but power users could route through Claude or GPT if they wanted.

The thing I want to push on strategically: the markdown-as-source-of-truth model works beautifully for solo use, but you mentioned the social/federated angle — people sharing what they're offering and looking for. Where does that live? Because if your canonical data is local markdown in a git repo, the "social" layer has to be something like: you selectively publish certain nodes or files to a shared space. Is that the vision? Like, your private graph stays private, but you can push certain structured "cards" — offers, requests, availability signals — out to a peer network?

## User

Basically, there are certain cards and stuff like that that are synced between the different users, but there's not really any kind of a central network source or truth. It's just peer to peer. 

Yeah, it's just pure to pure. My vision of it is, because I think this is a bit nicer for privacy, for the most part it's kind of like every single person sends out their key and query on a card thing. They would be git-ignored for each person, because if someone decides that they're git-ignored for each person and they're attracting your git repository, they are git-ignored for each person. This isn't really a security guarantee, but the idea here is that you don't have access to the full history of what a person's card thing will always look like. You don't have access to that. You have access to it at any given time, so somebody theoretically could do it, but the idea here is that you have your card. Your card is a part of your git repository that gets sent out to everybody peer-to-peer. This kind of stuff, I think, is just a bit nicer for privacy, so that if some person I don't know gets drunk one day and adds into their card thing some weird sex thing that they don't want public, they have 24 hours to change it. They have 24 hours to change it before it actually goes out. Similarly, depending on the context, if that thing actually does end up going out and that was immediately changed afterwards, it isn't actually stored anywhere unless anybody specifically was observing it. The idea is that there's a markdown file for each person that they give out. That markdown file is synced, which means everybody essentially in the future, I could see that there might be reasons to have a private and public card or something like that, but that's not what I'm going to work on right now because I feel like that will be a bit more complicated, but that is a plan in line.

I might also say that I would think that maybe you could undo your. That may be another privacy-helping feature. Essentially, you'd be able to return to an earlier version, and that will not require 24 hours.

## Claude

This is a really elegant privacy model. Let me make sure I have it right:

The card system:

Each user has a card — a markdown file in their repo — that represents what they're broadcasting to the network. It's git-ignored on their end (no local history), and it propagates peer-to-peer with a 24-hour delay buffer before it actually goes out. That delay is the privacy safety valve — drunk mistake at 11pm, sober correction by morning, nobody ever saw it. And if you catch it before it propagates, you can just revert immediately with no delay needed.

The privacy guarantee is soft but socially sufficient — it's not cryptographic, but it's "you'd have to be deliberately observing at exactly the right moment to capture something someone immediately corrected." That's a reasonable threat model for a rationalist community where bad actors aren't assumed.

The thing I find most interesting strategically here: this is actually a really nice inversion of how social networks normally work. Normally the network owns your history and you fight to delete things. Here, the absence of history is the default, and persistence requires active effort by an observer. That's worth articulating explicitly in the vision docs because it's a genuine differentiator.

One question: what determines the peer topology? If it's pure P2P with no central server, how does someone new find other cards to subscribe to? Is GitHub the bootstrap layer — like, you follow someone's GitHub and that's how you find their card? Or is there some other discovery mechanism you're envisioning?

## User

Oh, I, oh no, your card is getting tracked on your end because obviously you're going to want to use it for that, and it requires it to be done that way. For other people, the cards that they get from other people are getting ignored. They completely have the ability to use all the information in the cards, but they have to actively go out of their way to do so.

I would say that GitHub is something that could work for it. I would personally envision at a later date it changing, but I think GitHub and maybe something like following people on GitHub and people's usernames being their GitHub names would be an easy starting point. It would be an easy starting point where it's like you

## Claude

Right, that makes much more sense — your own card is tracked in your git history because you need that for the undo/revert feature. Other people's cards are git-ignored on your end, so you can use them in the moment but you're not building a surveillance archive of their history. The asymmetry is intentional and clean.

And the GitHub-as-identity bootstrap is pragmatic for exactly the right reasons. The rationalist/LessWrong/EA crowd already clusters on GitHub, already has usernames there, already thinks in terms of public profiles. You don't need to solve decentralized identity on day one — GitHub handles that, and you can swap it out later for something like DIDs or Nostr keys when the network matures enough to warrant it.

The thing I'd think about for that transition: if you design the identity layer as a thin abstraction from the start — "a user is identified by a canonical handle that currently resolves via GitHub" — then swapping the resolution mechanism later is much cleaner than if GitHub gets baked in everywhere.

The strategic question I'd be sitting with right now is sequencing. You have:

The private PRM (solo use, no network)

The card/P2P layer (social, federated)

The MCP server (agent interoperability)

These are three genuinely separable things. The private PRM is useful to one person with zero other users. The card layer requires at least two. The MCP server is infrastructure that makes everything more powerful.

My instinct is the private PRM is the right thing to make actually work first — because it's the thing that creates the graph that the cards are eventually a selective window into. You can't really design the card format well until you know what the underlying data model looks like in practice.

Does that sequencing match how you and the agent are thinking about it?

## User

Yeah, this is exactly what I'm intending. The PRM basically goes: well, I mean, the MCP server is something that exists; it would exist in step one, but its PRM first, then peer-to-peer second.  The idea is that people will use the PRM, and then they'll use the social elements because, why the fuck wouldn't you? It makes it more socially acceptable to use it at this point if social

## Claude

Yeah, that's a really smart adoption funnel. The PRM is the hook because it's immediately useful to you with zero network effects required. Then the social layer has a natural on-ramp: you already have your graph, your card is just a curated window into something you've already built. The friction to participating in the network is almost zero because the work is already done.

And there's a subtle social legitimacy point you're gesturing at — right now telling someone "I'm tracking our relationship in a CRM" sounds vaguely creepy. But if they're also using QueryKey, it becomes a shared norm. The peer-to-peer layer doesn't just add features, it changes the social meaning of using the tool at all. That's actually a really important insight for positioning.

The MCP server being there from day one is also smart because it means the agent story is coherent immediately — you're not locked to Gemma, sophisticated users can plug in whatever they want, and it frames QueryKey as infrastructure rather than just an app. That's important for the rationalist audience specifically, who are going to want to tinker.

One thing worth thinking about: the card format is going to do a lot of work. It's the thing that has to be human-readable markdown, machine-parseable for agents, expressive enough to capture "what I'm offering / looking for," and designed so that the 24-hour delay plus git-ignore model makes social sense. Getting that format right early — even just as a spec in the docs — seems like high leverage before the P2P layer gets built. Because once people start exchanging cards, the format ossifies fast.

Has the agent done any work on what a card actually looks like structurally?

## User

Well, it has a key and a query on it, obviously.

## Claude

Right, of course — the name is literally in the product name. Query and Key, matching the Q/K/V framing. Your card broadcasts what you're querying for (what you're looking for) and what your key is (what you're offering / who you are). Other people's agents attend over those when figuring out who might be relevant to connect with.

That's actually a really tight conceptual closure — the product metaphor isn't just branding, it's literally the data model. Your card is a Q/K pair, and the value gets computed locally when your agent matches your queries against other people's keys.

So the card is structurally simple: query fields and key fields, in markdown, with whatever frontmatter makes sense for machine parsing. The richness lives in what people actually put in those fields, not in a complicated schema.

What I'm curious about is whether there's a third element — something like context or constraints. Like, "I'm looking for X but only in Y context" or "I'm offering Z but only to people who have W." Or is the philosophy to keep the card intentionally flat and let the agent handle the nuanced matching locally?

## User

The bigger thing I would say is maybe people would have some kind of bio or something that is not in the matching thing but just gives a bit more context. Although personally, the way I've been imagining it is that people would oftentimes use their own personal websites to create their. He'll use their own personal websites generally for that kind of thing, but I think I don't think of that as being really part of somebody's card. I think of it as something else.

## Claude

Yeah that makes sense — the card stays lean and functional. Query, key, maybe a short bio or a link out to your personal site for context. The card isn't your profile, it's your signal. The richness of who you are lives elsewhere and you just point to it.

That's actually a healthier design than trying to make the card do everything. It keeps the P2P payload small, it keeps the matching logic clean, and it respects that people already have web presences they've curated. QueryKey doesn't need to compete with that or replicate it.

The distinction you're drawing — card vs. profile — is worth documenting explicitly because it'll come up constantly as the project grows. People will naturally want to add more to the card, and having a clear philosophical answer for why it stays minimal ("the card is your query and your key, everything else is your website") keeps scope creep from bloating the format.

It also means the card format is probably quite stable once defined, which is good for the P2P layer. Small, stable, versioned markdown files are much easier to sync reliably than rich evolving schemas.

Is there a value field at all in your thinking, or does V in the Q/K/V metaphor stay purely conceptual — the thing that gets computed by matching, not something anyone explicitly publishes?

## User

The value, I guess, would basically just be whatever it is that comes from their interactions. I think of the value as being something that exists more in the real world. I know this isn't quite right for the Transformer metaphor, but I don't think it needs to be. I don't think it needs to be, especially since these things are both metaphorical, because most of these things have a literal meaning. That's pretty good too.

## Claude

Yeah, that's actually a better framing than forcing the transformer metaphor to completion. In the transformer, V is just the weighted sum you compute — it's the output, not a stored thing. And in QueryKey, the "value" is literally what emerges from people actually connecting and doing things together in the real world. It's not stored in the system because it can't be — it's the relationship itself, the collaboration, the thing that happened.

So the metaphor holds at exactly the right level of abstraction: Q and K are the signals you broadcast and the system mediates, V is what you get out of it in reality. The product facilitates the attention mechanism, it doesn't try to capture or quantify the output. Which is honestly more philosophically honest than most social networks, which are obsessed with trying to measure and gamify the value generated.

That framing is also good for the rationalist audience specifically — they'll appreciate that the system is epistemically humble about what it can and can't represent. It tracks the legible stuff, it facilitates matching, and it gets out of the way when actual human stuff happens.

So the core of the card is really just two things: here's what I'm looking for, here's what I bring. Clean, minimal, honest. That's a strong foundation.

## User

Yeah, the basic thing is, essentially, the card isn't supposed to give a whole lot. I'm mostly imagining that it attracts people. It isn't really trying to attract that much stuff, basically.

For the most part, my idea is that people's personal websites or whatever like that should be more or less the thing that's more or less the actual source of truth here. Realistically, people are probably not going to be necessarily the best, and people are probably going to be not necessarily even the best at writing out a lot of stuff.

That's part of the reason why they're not specifically using agents. They want to try to get people to write their key and query based off of their agents and also using agents to do the analysis, because I feel like people are not really like that. I feel like people are generally not that good at kind of picking out, particularly, what they're strong at, particularly younger people.

## Claude

Yeah, that's a really insightful observation. The agent-assisted card generation is actually solving a real problem — most people are genuinely bad at articulating their own value proposition, especially younger people who haven't had enough feedback loops to know what they're actually good at yet. Having the agent interview you or analyze your existing writing and suggest your key is much more likely to produce an accurate card than asking someone to fill out a form.

And it closes a nice loop — the agent has been building your PRM, it's been watching your conversations and commitments and what you're drawn to and what you're good at following through on. It's actually in a better position than you are to write a first draft of your key. You review and edit, but the heavy lifting is done.

Same with the query — people often don't know what they're looking for until they see it. The agent can notice patterns in what you respond to, what you reach out about, what kinds of interactions energize you, and surface that as a query.

So the card becomes something the agent drafts from observing you over time, you curate and approve, and then it goes out. That's a much more honest representation than a self-reported profile.

The personal website as source of truth is also smart because it already exists, it's already how people present themselves publicly, and linking to it from the card means the card stays minimal without losing depth. The card is the hook, the website is the substance.

Has there been any thinking about how the agent actually generates the initial card draft — is it a conversation, an analysis of your existing data, or both?

## User

It'll probably be some kind of a prog thing. There's probably going to be some kind of hs-progged stuff that may be in hs.md or something that is stored within your Git repository. You could use system prog, but I would say that within your Git repository is going to be the thing that probably is going to be most effective. Within your Git repository, because the idea is that you have the ability to more or less change this.

## Claude

Right, so the agent's behavior is itself configurable via markdown files in your repo — like a `hs.md` or similar that contains the prompts or heuristics that guide how it generates your card, how it analyzes your relationships, what it pays attention to. That's a really clean design because it means the agent's "personality" and priorities are just files you can edit, version control, and share if you want.

It's also very much in the spirit of the whole project — your data, your config, your git repo, all of it transparent and auditable. The agent isn't a black box doing mysterious things to your social graph, its instructions are sitting right there in the repo next to your markdown files.

And it creates an interesting second-order thing where the rationalist community will inevitably start sharing and comparing their `hs.md` configurations — "here's the heuristics I use for card generation, here's what I tell the agent to look for" — which is its own kind of knowledge sharing that happens organically on top of the tool.

The name `hs.md` — is that short for something specific, or is that just a working name?

## User

I said agents.md

## Claude

Ah, `agents.md` — that makes more sense, cleaner name. So that's the file that configures how your local agent behaves — the prompts, heuristics, priorities. Essentially your agent's instructions living as a markdown file in your repo that you own and can edit.

That's actually a really powerful primitive because it means QueryKey isn't opinionated about what your agent does beyond the basics. Two people could have radically different `agents.md` files and get radically different behavior out of the same system. One person optimizes for professional networking, another for finding collaborators on creative projects, another for maintaining close friendships. The tool adapts to the user's actual goals rather than imposing a one-size-fits-all relationship model.

It also means the MCP server exposure is essentially executing against whatever's in `agents.md` — so when Claude or Gemma or any other model connects, it's operating within the behavioral envelope you've defined for yourself. Which is a nice safety and customization property.

Is `agents.md` something you're imagining as a single file or more of an `agents/` directory as it gets more complex?

## User

Could be whatever.

## Claude

Fair enough, that's an implementation detail that'll sort itself out naturally as it gets built. No need to over-specify it now.

Stepping back — you've got a pretty coherent architecture at this point. Local markdown as source of truth, git for history, Loca for the derived graph, MCP server for agent interoperability, `agents.md` for behavioral config, and the card system as a minimal P2P broadcast layer. The pieces fit together well and each one is independently useful.

What's the thing you're most uncertain about strategically right now? Is it technical, or is it more about how you position and grow it within the rationalist community?

## User

Honestly I feel like it just needs users first and we can do more stuff with it over time

## Claude

Yeah, that's the right instinct. Don't over-engineer the social layer before anyone's actually using the PRM. Get the solo use case working well enough that you are using it daily, then maybe a handful of people from your network, and let the actual usage patterns tell you what matters next.

The rationalist/LessWrong crowd is actually a pretty good first cohort for this because they're unusually tolerant of early, rough tools if the underlying idea resonates, and they'll give you honest specific feedback rather than just churning silently.

Is there anything blocking the PRM being usable for you personally right now, or is the agent making good progress toward that?

## User

I mean idk I think it’s pretty complete

## Claude

Then honestly just use it. That's the whole next step — eat your own cooking, see what's annoying, see what's missing, see what you actually reach for. No amount of strategic discussion beats a week of real use.

Who in your network do you think you'd onboard first?
