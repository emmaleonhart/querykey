# Secretarybird Pivot

A real-time knowledge graph and task extraction system that takes in messy, unstructured communication — Discord chatlogs, screenshots, voice notes, recorded conversations, pasted text, bot feeds — and extracts structured tasks, assignments, and contradictions from it.

**We don't impose structure. We extract it from the chaos that already exists.**

Small businesses run on informal communication. Tasks get assigned over Discord, in hallway conversations, through voice messages, across group chats. Secretarybird watches all of it and builds a living task board without asking anyone to change how they work.

Built with **Flutter** (mobile, desktop, web). Analysis powered by **OpenClaw**.

## The Problem

A very large amount of money is lost in businesses due to contradictory or vague verbal instructions. Tasks get assigned informally and people walk away with different understandings of who is doing what. There is no single source of truth for informal task delegation — and nobody is going to start filing Jira tickets for what their boss said in passing.

## The Solution

Secretarybird accepts any form of communication input and builds a continuously updated knowledge graph on the server. OpenClaw processes the incoming information and extracts:

- **Who** was assigned a task
- **What** the task is
- **When** it was assigned and any deadlines mentioned
- **Contradictions** — when a newer instruction conflicts with a previous one
- **Ambiguities** — when instructions are vague enough to cause confusion

The result is a Jira-style task board that populates itself automatically from natural conversation, with built-in contradiction detection.

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

**Secretarybird Server** can run locally on a machine or deployed to the cloud. It handles ingestion, storage, real-time sync, and coordinates with OpenClaw for AI analysis.

**Secretarybird Mobile** is the Flutter app — runs on phone, desktop, and web. Thin client that streams data to the server and renders the graph.

## Tech Stack

- **Frontend**: Flutter (iOS, Android, Desktop, Web) — single codebase
- **Server**: Secretarybird Server (local or cloud)
- **AI Analysis**: OpenClaw
- **Audio**: On-device recording, server-side transcription
- **Real-time**: WebSocket connections for live feed updates and graph changes

## Project Status

**Planning phase.** See `/docs` for detailed planning documents.

History from the original secretarybird repo (Electron-based) will be incorporated later. The socket infrastructure from that project is particularly relevant.
