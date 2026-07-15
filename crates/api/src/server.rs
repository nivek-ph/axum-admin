use tracing::info;

use crate::AppState;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub listen_addr: String,
    pub public_url: String,
}

pub async fn serve(config: ServerConfig, state: AppState) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(&config.listen_addr).await?;
    let listen_addr = listener.local_addr()?;
    info!(
        %listen_addr,
        swagger_url = %format!("{}/swagger-ui/", config.public_url.trim_end_matches('/')),
        "api server listening"
    );
    axum::serve(listener, crate::router(state)).await
}
