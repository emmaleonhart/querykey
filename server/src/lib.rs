//! QueryKey server (Rust). Port of server-go-old/ with the graph store
//! on Loca/SutraDB instead of the dead Fuseki stub. Modules mirror the
//! Go layout (see server-go-old/README.md for the mapping).

pub mod api;
pub mod card;
pub mod config;
pub mod discord;
pub mod graph;
pub mod identity;
pub mod ingest;
pub mod mcp;
pub mod models;
pub mod openclaw;
pub mod vault;
pub mod wikilink;
pub mod workflow;
pub mod ws;
