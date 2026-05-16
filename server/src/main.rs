//! Port of server-go-old/cmd/secretarybird/main.go: wire config ->
//! bridge -> graph (Loca) -> hub -> pipeline -> (optional Discord) ->
//! axum HTTP server with graceful shutdown.

use std::sync::Arc;

use querykey_server::api::{build_router, AppState};
use querykey_server::config::Config;
use querykey_server::discord::DiscordBot;
use querykey_server::graph::{memory::InMemoryGraph, GraphStore};
use querykey_server::ingest::Pipeline;
use querykey_server::openclaw::Bridge;
use querykey_server::vault::Vault;
use querykey_server::ws::Hub;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("[querykey] starting up...");
    let cfg = Config::load();

    // Local AI agent bridge.
    let bridge = Arc::new(Bridge::new(
        &cfg.openclaw_gateway_url,
        &cfg.openclaw_agent_id,
        &cfg.openclaw_token,
    ));
    let st = bridge.detect().await;
    if st.available {
        tracing::info!(
            "[querykey] agent gateway connected: {} (agent: {})",
            st.gateway_url,
            st.agent_id
        );
    } else {
        tracing::warn!("[querykey] agent gateway not available: {}", st.error);
        let b = bridge.clone();
        tokio::spawn(async move { b.ensure_gateway().await });
    }

    // Graph store: Loca/SutraDB when built with --features loca,
    // otherwise the in-memory fallback (always builds & runs).
    let graph: Arc<dyn GraphStore> = select_graph(&cfg);
    match graph.ping().await {
        Ok(()) => tracing::info!("[querykey] graph store: {}", graph.backend()),
        Err(e) => tracing::warn!(
            "[querykey] graph store {} not ready: {e} (continuing)",
            graph.backend()
        ),
    }
    let _ = graph.ensure_dataset().await;

    // Canonical markdown vault (the store of record).
    let vault = Arc::new(Vault::open(&cfg.vault_dir)?);
    tracing::info!("[querykey] vault: {}", vault.root().display());

    // The graph is a DERIVED index — rebuild it from the vault on
    // startup so it always reflects the canonical files.
    {
        let people = vault.list_persons();
        let tasks = vault.list_tasks();
        for p in &people {
            let _ = graph.store_person(p).await;
        }
        for t in &tasks {
            let _ = graph.store_task(t).await;
        }
        tracing::info!(
            "[querykey] projected vault → graph: {} person(s), {} task(s)",
            people.len(),
            tasks.len()
        );
    }

    // WebSocket hub + ingestion pipeline.
    let hub = Arc::new(Hub::new(bridge.clone()));
    let pipeline = Arc::new(Pipeline::new(
        bridge.clone(),
        graph.clone(),
        vault.clone(),
        hub.clone(),
    ));

    // Optional Discord bot (not yet ported; no-op when absent).
    let _bot = DiscordBot::try_start(
        &cfg.discord_token,
        &cfg.discord_guild_ids,
        cfg.discord_batch_interval,
    );

    let state = Arc::new(AppState {
        bridge: bridge.clone(),
        graph: graph.clone(),
        vault: vault.clone(),
        hub: hub.clone(),
        pipeline,
    });
    let app = build_router(state);

    let addr = format!("{}:{}", cfg.host, cfg.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("[querykey] server listening on {addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("[querykey] shutting down...");
    bridge.stop_gateway();
    tracing::info!("[querykey] shutdown complete");
    Ok(())
}

#[cfg(feature = "loca")]
fn select_graph(cfg: &Config) -> Arc<dyn GraphStore> {
    match querykey_server::graph::loca::LocaGraph::open(&cfg.loca_db_path) {
        Ok(g) => Arc::new(g),
        Err(e) => {
            tracing::warn!(
                "[querykey] Loca open failed ({e}); falling back to in-memory"
            );
            Arc::new(InMemoryGraph::new())
        }
    }
}

#[cfg(not(feature = "loca"))]
fn select_graph(_cfg: &Config) -> Arc<dyn GraphStore> {
    Arc::new(InMemoryGraph::new())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.ok();
    };
    #[cfg(unix)]
    let term = async {
        use tokio::signal::unix::{signal, SignalKind};
        if let Ok(mut s) = signal(SignalKind::terminate()) {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = term => {},
    }
}
