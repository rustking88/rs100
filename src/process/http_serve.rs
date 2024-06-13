use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Router,
};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::net::TcpListener;
use tracing::{info, warn};
use tower_http::services::ServeDir;

#[derive(Debug)]
struct HttpServeState {
    path: PathBuf,
}

pub async fn process_http_serve(path: PathBuf, port: u16) -> Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Serving {:?} on {}", path, addr);

    let state = HttpServeState { path: path.clone() };
    let dir_service = ServeDir::new(path.clone())
    .append_index_html_on_directories(true)
    .precompressed_br()
    .precompressed_deflate()
    .precompressed_gzip()
    .precompressed_zstd();
    //axum route
    let router = Router::new()
        .route("/*path", get(index_handler))
        .route("/api", post(api_handler))
        .nest_service("/tower", ServeDir::new(path.clone()))
        .with_state(Arc::new(state));

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}

async fn index_handler(
    State(state): State<Arc<HttpServeState>>,
    Path(path): Path<String>,
) -> (StatusCode, String) {
    let p = std::path::Path::new(&state.path).join(path);
    info!("Serving file: {}", p.display());
    if !p.exists() {
        (
            StatusCode::NOT_FOUND,
            format!("File {} not found", p.display())
        )
    } else {
        match tokio::fs::read_to_string(p).await {
            Ok(content) => {
                info!("Read {} bytes", content.len());
                (StatusCode::OK, content)
            }
            Err(e) => {
                warn!("Error reading file: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()) 
            }
        }
    }
}

async fn api_handler() -> &'static str {
    "API endpoint"
}

#[cfg(test)]

mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_file_handler() {
        let state = Arc::new(HttpServeState {
            path: PathBuf::from("."),
        });
        let (status, content) = index_handler(
            State(state), Path("cargo.toml".to_string())).await;
        assert_eq!(status, StatusCode::OK);
        assert!(content.trim().starts_with("[package]"));
    }

}

