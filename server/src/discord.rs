//! Port stub of server-go-old/internal/discord/bot.go.
//!
//! The Go bot used discordgo (gateway connect, channel monitoring,
//! hourly batch processing, DM follow-ups). The Rust port would use
//! `serenity`/`twilight`. That is a substantial sub-port; until it is
//! done the bot is a no-op, exactly as the Go server treated it as
//! optional ("DISCORD_TOKEN not set, bot disabled").
//!
//! TODO(port): full bot — see server-go-old/internal/discord/bot.go.

pub struct DiscordBot;

impl DiscordBot {
    /// Returns Some(bot) only when a token is configured. Currently
    /// always logs that the bot is not yet ported and returns None.
    pub fn try_start(token: &str, guilds: &[String], batch_interval: i64) -> Option<DiscordBot> {
        if token.is_empty() {
            tracing::info!("[discord] DISCORD_TOKEN not set, bot disabled");
            return None;
        }
        tracing::warn!(
            "[discord] token present, {} guild(s), {}min batch — but the \
             Discord bot is not yet ported to Rust (see discord.rs / \
             server-go-old). Continuing without Discord.",
            guilds.len(),
            batch_interval
        );
        None
    }

    pub fn stop(&self) {}
}
