# Secretarybird Pivot

A real-time knowledge graph and task extraction system that monitors communication streams — Discord, Slack, phone conversations, chat logs, and social media — and organizes them into structured, actionable task assignments.

Built with **Flutter** (mobile + desktop) and powered by **OpenClaw**.

## The Problem

A very large amount of money is lost in businesses due to contradictory or vague verbal instructions. Tasks get assigned informally across meetings, Discord channels, Slack threads, and hallway conversations. People walk away with different understandings of who is doing what. There is no single source of truth for informal task delegation.

## The Solution

Secretarybird watches all communication channels in real time and builds a continuously updated knowledge graph on the server. An AI processes incoming streams of information and extracts:

- **Who** was assigned a task
- **What** the task is
- **When** it was assigned and any deadlines mentioned
- **Contradictions** — when a newer instruction conflicts with a previous one
- **Ambiguities** — when instructions are vague enough to cause confusion

The result is a Jira-style task board that populates itself automatically from natural conversation, with built-in contradiction detection.

## Information Sources

| Source | Method |
|---|---|
| Discord servers | Bot / webhook integration, real-time message streaming |
| Slack workspaces | Slack app integration, real-time event API |
| Verbal conversations | Phone app records and streams audio to server for transcription |
| Chat logs | Paste or upload text logs for batch processing |
| Social media posts | Read and ingest posts from configured accounts |

## Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│                   Flutter Frontend                   │
│  ┌──────────────┐  ┌─────────────────────────────┐  │
│  │  Phone App   │  │     Desktop / Web App       │  │
│  │ - Mic record │  │ - Knowledge graph view      │  │
│  │ - Graph view │  │ - Task board (Jira-style)   │  │
│  └──────┬───────┘  │ - Contradiction alerts      │  │
│         │          │ - Feed management            │  │
│         │          └─────────────┬───────────────┘  │
└─────────┼────────────────────────┼──────────────────┘
          │                        │
          ▼                        ▼
┌─────────────────────────────────────────────────────┐
│                  OpenClaw Backend                     │
│  ┌────────────┐  ┌─────────────┐  ┌──────────────┐ │
│  │ Ingest     │  │ AI Pipeline │  │ Knowledge    │ │
│  │ Service    │  │ - NLP/LLM   │  │ Graph Store  │ │
│  │ - Discord  │  │ - Entity    │  │ - Entities   │ │
│  │ - Slack    │  │   extraction│  │ - Tasks      │ │
│  │ - Audio    │  │ - Task      │  │ - Relations  │ │
│  │ - Paste    │  │   detection │  │ - Conflicts  │ │
│  │ - Social   │  │ - Conflict  │  │              │ │
│  │            │  │   detection │  │              │ │
│  └────────────┘  └─────────────┘  └──────────────┘ │
└─────────────────────────────────────────────────────┘
```

## Phone App

The mobile app serves two core functions:

1. **Listen** — Record ambient/meeting audio and stream it to the server for real-time transcription and task extraction
2. **View** — Display the knowledge graph and task board, browse contradictions, check assignments

## Tech Stack

- **Frontend**: Flutter (iOS, Android, Desktop, Web)
- **Backend**: OpenClaw
- **AI/NLP**: LLM-based entity extraction, task detection, contradiction analysis
- **Audio**: On-device recording, server-side transcription (Whisper or equivalent)
- **Graph Store**: To be determined — likely a graph database (Neo4j, etc.) or a structured relational model
- **Real-time**: WebSocket connections for live feed updates and graph changes

## Project Status

**Planning phase.** See `/docs` for detailed planning documents.
