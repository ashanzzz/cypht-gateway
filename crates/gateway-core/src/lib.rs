//! Stable types shared by every Cypht Gateway surface.
//!
//! AI-maintenance rule: protocol and provider implementation details belong in adapters.
//! Public REST/MCP/CLI contracts should depend on the types in this crate instead.

mod build_info;
mod error;
mod ids;
mod models;

pub use build_info::BuildInfo;
pub use error::{ApiErrorBody, ApiErrorDetail, GatewayError, GatewayResult};
pub use ids::{DecodedObjectId, ObjectIdCodec, ObjectKind};
pub use models::*;
