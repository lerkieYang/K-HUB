pub mod memory;
pub mod knowledge;
pub mod agents;
pub mod export;
pub mod mcp;
pub mod ai;
pub mod filesystem;
pub mod memory_pull;
pub mod device;
pub mod ingest;
pub mod artifact;
pub mod context;
pub mod data_source;
pub mod sync;
pub mod workspace;

use axum::Router;
use crate::db::Database;

pub fn routes() -> Router<Database> {
    Router::new()
        .merge(memory::routes())
        .merge(knowledge::routes())
        .merge(agents::routes())
        .merge(export::routes())
        .merge(mcp::routes())
        .merge(ai::routes())
        .merge(filesystem::routes())
        .merge(memory_pull::routes())
        .merge(device::routes())
        .merge(ingest::routes())
        .merge(artifact::routes())
        .merge(context::routes())
        .merge(data_source::routes())
        .merge(sync::routes())
        .merge(workspace::routes())
}
