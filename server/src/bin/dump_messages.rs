//! Port stub of server-go-old/cmd/dump-messages/main.go.
//!
//! R4-6 triage: the Go tool is **wholly Discord-coupled** — it opens a
//! discordgo session from DISCORD_TOKEN, walks guilds/channels, and
//! dumps messages to `dev_scheduling/receipts/discord`. There is no
//! non-Discord part to port. It therefore belongs with the
//! **deprioritized Discord work in `todo.md` Phase Z**, not Round 4.
//! It deliberately does NOT block deleting `server-go-old/`.
//!
//! When Phase Z is picked up, port this against the feature-gated
//! serenity bot (`server/src/discord.rs`, `--features discord`).

fn main() {
    eprintln!(
        "dump-messages: deferred with Discord (todo.md Phase Z; \
         wholly Discord-coupled — see server-go-old/cmd/dump-messages). \
         Exiting 0 so CI scripts don't hard-fail."
    );
}
