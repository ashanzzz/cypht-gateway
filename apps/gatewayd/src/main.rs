use axum::{
    extract::State,
    http::StatusCode,
    routing::get,
    Json, Router,
};
use gateway_core::BuildInfo;
use serde::Serialize;
use std::{env, net::SocketAddr};
use tower_http::trace::TraceLayer;
use tracing::info;

#[derive(Clone)]
struct AppState {
    build: BuildInfo,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

async fn health(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "ok",
            version: state.build.version,
        }),
    )
}

async fn version(State(state): State<AppState>) -> Json<BuildInfo> {
    Json(state.build)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "gatewayd=info,tower_http=info".into()),
        )
        .init();

    let bind = env::var("GATEWAY_BIND").unwrap_or_else(|_| "0.0.0.0:8080".to_owned());
    let addr: SocketAddr = bind.parse()?;
    let build = BuildInfo::current();

    let app = Router::new()
        .route("/healthz", get(health))
        .route("/api/v1/meta/version", get(version))
        .layer(TraceLayer::new_for_http())
        .with_state(AppState { build: build.clone() });

    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(%addr, version = build.version, git = build.git_short_sha, "cypht-gateway started");
    axum::serve(listener, app).await?;
    Ok(())
}
