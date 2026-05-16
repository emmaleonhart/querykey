//! Discord bot. Port of server-go-old/internal/discord/bot.go.
//!
//! Behind the `discord` cargo feature (default OFF, like `loca`) this
//! is a real `serenity` client: connect, log on ready, receive
//! messages. Without the feature it is the original no-op stub, so the
//! default build never depends on serenity and stays green.
//!
//! TODO(port): the deeper Go behavior — per-channel monitoring filters,
//! hourly batch processing into the ingest pipeline, and outbound DM
//! follow-ups (see server-go-old/internal/discord/bot.go).

#[cfg(not(feature = "discord"))]
mod imp {
    pub struct DiscordBot;

    impl DiscordBot {
        pub fn try_start(
            token: &str,
            guilds: &[String],
            batch_interval: i64,
        ) -> Option<DiscordBot> {
            if token.is_empty() {
                tracing::info!("[discord] DISCORD_TOKEN not set, bot disabled");
                return None;
            }
            tracing::warn!(
                "[discord] token present ({} guild(s), {}min batch) but the \
                 server was built without --features discord; continuing \
                 without Discord.",
                guilds.len(),
                batch_interval
            );
            None
        }

        pub fn stop(&self) {}
    }
}

#[cfg(feature = "discord")]
mod imp {
    use serenity::all::{Client, Context, EventHandler, GatewayIntents, Message, Ready};
    use serenity::async_trait;
    use tokio::task::JoinHandle;

    struct Handler {
        guilds: Vec<String>,
    }

    #[async_trait]
    impl EventHandler for Handler {
        async fn ready(&self, _ctx: Context, ready: Ready) {
            tracing::info!(
                "[discord] connected as {} (monitoring {} configured guild(s))",
                ready.user.name,
                self.guilds.len()
            );
        }

        async fn message(&self, _ctx: Context, msg: Message) {
            if msg.author.bot {
                return;
            }
            // Minimal real ingest surface: log every human message.
            // TODO(port): buffer + hourly batch into the ingest
            // pipeline, and DM follow-ups (bot.go).
            tracing::info!(
                "[discord] {} #{} <{}>: {}",
                msg.guild_id
                    .map(|g| g.to_string())
                    .unwrap_or_else(|| "DM".into()),
                msg.channel_id,
                msg.author.name,
                msg.content
            );
        }
    }

    pub struct DiscordBot {
        handle: JoinHandle<()>,
    }

    impl DiscordBot {
        pub fn try_start(
            token: &str,
            guilds: &[String],
            batch_interval: i64,
        ) -> Option<DiscordBot> {
            if token.is_empty() {
                tracing::info!("[discord] DISCORD_TOKEN not set, bot disabled");
                return None;
            }
            tracing::info!(
                "[discord] starting serenity client ({} guild(s), {}min batch)",
                guilds.len(),
                batch_interval
            );
            let token = token.to_string();
            let guilds = guilds.to_vec();
            // Spawned on the current tokio runtime (main is #[tokio::main]).
            let handle = tokio::spawn(async move {
                let intents = GatewayIntents::GUILD_MESSAGES
                    | GatewayIntents::DIRECT_MESSAGES
                    | GatewayIntents::MESSAGE_CONTENT;
                match Client::builder(&token, intents)
                    .event_handler(Handler { guilds })
                    .await
                {
                    Ok(mut client) => {
                        if let Err(e) = client.start().await {
                            tracing::warn!("[discord] client stopped: {e}");
                        }
                    }
                    Err(e) => tracing::warn!("[discord] client build failed: {e}"),
                }
            });
            Some(DiscordBot { handle })
        }

        pub fn stop(&self) {
            self.handle.abort();
        }
    }
}

pub use imp::DiscordBot;
