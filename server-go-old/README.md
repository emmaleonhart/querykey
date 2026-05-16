# `server-go-old/` — archived Go server (deprecated)

This is the **previous Go implementation** of the QueryKey server. It
is **deprecated and no longer the build target.** The server is being
**rewritten in Rust** (now in [`../server/`](../server/)) with the
graph store on **Loca/SutraDB** instead of the dead Fuseki stub.

It is kept here, not deleted, as the **reference implementation** for
the port: the Rust server's modules mirror this layout, and porting
notes/TODOs in `../server/` point back to specific files here.

| Go (here) | Rust port (`../server/`) |
|---|---|
| `internal/config/` | `src/config.rs` |
| `internal/models/` | `src/models.rs` |
| `internal/openclaw/` | `src/openclaw/` |
| `internal/ingest/` | `src/ingest.rs` |
| `internal/ws/` | `src/ws.rs` |
| `internal/discord/` | `src/discord.rs` |
| `internal/api/` | `src/api/` |
| `internal/graph/fuseki.go` | `src/graph/` → **Loca**, not Fuseki |
| `cmd/secretarybird/` | `src/main.rs` |
| `cmd/dump-messages/` | `src/bin/dump_messages.rs` |

Once the Rust server reaches parity this directory will be deleted
(the same way `secretarybird-old/` was — full history stays in git).
Do not add new code here.
