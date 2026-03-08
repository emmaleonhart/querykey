# Secretarybird Pivot

An AI secretary that actively manages your team's communication. It ingests messy, unstructured input — verbal instructions, Discord chats, voice notes, screenshots, recorded meetings — extracts structured tasks and contradictions, and then **conversationally follows up with people** to verify understanding, resolve conflicts, and keep everyone aligned.

**The secretarybird is here to serve you. You don't serve it.**

Built with **Flutter** (mobile, desktop, web). Analysis powered by **OpenClaw**.

## Philosophy

Tools like Jira, Git, and Azure DevOps often become performative work — people spend more time maintaining the tool than doing the actual work. Status updates become theater. Ticket grooming becomes a job in itself.

Secretarybird takes the opposite approach:
- **The tool serves you, not the other way around.** You never need to fill out a form, file a ticket, or update a status. The system figures it out from your actual communication.
- **We connect to existing tools, not replace them.** Secretarybird integrates with Git, Jira, Azure DevOps, and other project management tools. It can sync extracted tasks into those systems so teams that need formal tracking get it automatically — without anyone manually entering data.
- **We are not a no-code solution. We are an "AI writes the code" solution.** The system doesn't give you drag-and-drop workflow builders. It uses AI to do the actual hard work — parsing unstructured input, resolving entities, detecting contradictions, writing the integrations. The intelligence is in the AI, not in a visual editor.

## The Problem

A very large amount of money is lost in businesses due to contradictory or vague verbal instructions. Tasks get assigned informally and people walk away with different understandings of who is doing what. There is no single source of truth for informal task delegation — and nobody is going to start filing Jira tickets for what their boss said in passing.

The best way to know what someone thinks they're supposed to do is to ask them. No tool does this today.

## The Solution

Secretarybird is an AI secretary. It doesn't just passively organize — it **actively engages with your team**.

**Ingest**: It accepts any form of communication input (verbal, text, screenshots, bot feeds) and builds a continuously updated knowledge graph.

**Extract**: OpenClaw processes incoming information and extracts who was assigned what, when, by whom, and flags contradictions and ambiguities.

**Follow up**: This is the differentiator. Secretarybird **talks to people**. When it detects a contradiction — "in this conversation you were told to finish the video by 10 PM, but in this other one you were told 8 AM" — it messages the relevant person and asks which one they're planning to follow. It asks simple questions, listens, records the answers, and exchanges information between team members.

**Notify**: It sends notifications based on scheduling, deadlines, and extracted commitments. It doesn't wait for someone to check a dashboard.

**Verify**: The primary purpose is **verifiability of verbal instructions**. When your boss tells you something in passing, the system captures it, confirms it with you, and creates a clear record that both parties can reference.

### How It Communicates

The AI sends **short, simple messages**. The goal is not to explain things. The goal is to:
- Ask a clear question
- Listen to the answer
- Record it
- Pass relevant information between team members

It acts like a good secretary — concise, organized, never the center of attention. It asks "Are you doing X or Y?" not "Based on my analysis of the situational context, I've identified a potential discrepancy..."

### Epistemic Humility

The AI does not need to be perfect. It needs to know when it's not sure.

Expected accuracy model:
- **~75%** — correctly extracted from data feeds, no follow-up needed
- **~20%** — extracted incorrectly, but the system recognizes the uncertainty and asks a clarifying question, getting to the right answer
- **~5%** — genuine errors where the system is wrong and doesn't catch it

This is a success. The reason the system can tolerate imperfect extraction is precisely because it has the ability to ask. A passive tool that's wrong 25% of the time is useless. An active secretary that's wrong 25% of the time but catches most of its mistakes by asking? That's a good secretary.

Everything the system records is visible and auditable. All context is provided in places where people can view it — followable calendars, task boards, conversation logs. Nothing is hidden. If the AI got something wrong, anyone can see it and correct it.

### Voice Learning

Secretarybird learns the voices of team members. Over time, it identifies who is speaking in recorded meetings and conversations without manual tagging. This makes verbal instruction capture seamless — record a conversation, and the system knows who said what.

## Unstructured Input — Anything Goes

The core design principle is that **any form of input works**. Users should never feel like they need to format something correctly. The system handles:

| Input Type | How It Works |
|---|---|
| Discord bot | Real-time message streaming from monitored channels |
| Pasted Discord chatlogs | Copy-paste a conversation, AI parses it |
| Screenshots | Paste or upload a screenshot of a conversation, OCR + AI extracts content |
| Voice notes | Record a voice memo explaining what someone told you |
| Recorded conversations | Phone app records meetings/calls, streams audio for transcription |
| Pasted text | Any freeform text — email forwards, notes, whatever |
| Slack bot | Real-time message streaming from monitored workspaces |

A Discord bot recording a channel in real time and a user pasting a screenshot of that same channel are equally valid inputs. The system normalizes everything.

## System Architecture

```
┌─────────────────────────────────────────────────────────┐
│              Secretarybird Mobile / Desktop              │
│                    (Flutter App)                         │
│  ┌──────────────────┐  ┌────────────────────────────┐   │
│  │  Mobile Features  │  │    Shared Features         │   │
│  │ - Mic recording   │  │ - Knowledge graph view     │   │
│  │ - Voice notes     │  │ - Task board (Jira-style)  │   │
│  │ - Push notifs     │  │ - Contradiction alerts     │   │
│  └────────┬─────────┘  │ - Unstructured import       │   │
│           │             │   (paste, screenshot, text) │   │
│           │             └──────────────┬─────────────┘   │
└───────────┼────────────────────────────┼─────────────────┘
            │          WebSocket         │
            ▼                            ▼
┌─────────────────────────────────────────────────────────┐
│              Secretarybird Server                         │
│           (local or cloud deployment)                    │
│                                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │ Ingest       │  │ Knowledge    │  │ Real-time    │  │
│  │ Service      │  │ Graph Store  │  │ Sync         │  │
│  │ - Any input  │  │ - Entities   │  │ - WebSocket  │  │
│  │ - Normalize  │  │ - Tasks      │  │ - Graph diff │  │
│  │ - OCR        │  │ - Relations  │  │ - Push       │  │
│  │ - Transcribe │  │ - Conflicts  │  │              │  │
│  └──────┬───────┘  └──────────────┘  └──────────────┘  │
│         │                                                │
│         ▼                                                │
│  ┌──────────────┐                                        │
│  │ OpenClaw     │  AI analysis: entity extraction,       │
│  │ (Analysis)   │  task detection, contradiction         │
│  │              │  detection, ambiguity scoring           │
│  └──────────────┘                                        │
└─────────────────────────────────────────────────────────┘
```

**Secretarybird Server** can run locally on a machine or deployed to the cloud. It handles ingestion, storage, real-time sync, outbound messaging, and coordinates with OpenClaw for AI analysis.

**Secretarybird Mobile** is the Flutter app — runs on phone, desktop, and web. Where team members receive questions, view the live task feed, and record conversations.

## Tech Stack

- **Frontend**: Flutter (iOS, Android, Desktop, Web) — single codebase
- **Server**: Secretarybird Server (local or cloud)
- **AI Analysis**: OpenClaw
- **Audio**: On-device recording, server-side transcription
- **Real-time**: WebSocket connections for live feed updates and graph changes

## Project Status

**Planning phase.** See `/docs` for detailed planning documents.

History from the original secretarybird repo (Electron-based) will be incorporated later. The socket infrastructure from that project is particularly relevant.
