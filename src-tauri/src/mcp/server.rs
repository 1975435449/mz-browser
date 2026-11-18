//! 本机 HTTP 控制服务：只监听 127.0.0.1，所有请求先经过 security::guard（Host、Origin、令牌、大小），
//! 接口见 http_api。模块名沿用 mcp，但已经不再提供 MCP 协议。
use std::{net::SocketAddr, sync::Arc};

use anyhow::{Context, Result};
use axum::{Router, extract::State, middleware, response::IntoResponse, routing::get};
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};

use crate::mcp::{bridge::LocalApiBridge, config::McpServerRuntimeConfig, security};

#[derive(Clone)]
pub struct McpServer {
    bridge: Arc<LocalApiBridge>,
    config: Arc<McpServerRuntimeConfig>,
}

impl McpServer {
    pub fn new(config: &McpServerRuntimeConfig) -> Self {
        Self {
            bridge: Arc::new(LocalApiBridge::new()),
            config: Arc::new(config.clone()),
        }
    }

    pub fn bridge(&self) -> &LocalApiBridge {
        self.bridge.as_ref()
    }

    pub fn config(&self) -> &McpServerRuntimeConfig {
        &self.config
    }

    pub async fn require_session(&self) -> Result<(), super::error::McpToolError> {
        let app = crate::app::handle::get_app_handle()
            .map_err(|_| super::error::McpToolError::invalid_params("Application is not ready"))?;
        let current = super::config::load_config(&app).map_err(|_| {
            super::error::McpToolError::internal("Cannot validate current HTTP API policy")
        })?;
        if !current.enabled || current != self.config.access {
            return Err(super::error::McpToolError::forbidden(
                "HTTP API policy changed; restart the HTTP API",
            ));
        }
        Ok(())
    }

    /// Guard for the management tier (environments, groups, proxies, tags, kernels).
    pub async fn require_manage(&self) -> Result<(), super::error::McpToolError> {
        self.require_session().await?;
        if !self.config.access.manage_tools_enabled {
            return Err(super::error::McpToolError::forbidden(
                "Management tools are disabled",
            ));
        }
        Ok(())
    }

    /// Guard for the page-action tier; also needs page reading.
    pub async fn require_act(&self) -> Result<(), super::error::McpToolError> {
        self.require_session().await?;
        let access = &self.config.access;
        if !access.page_tools_enabled || !access.act_tools_enabled {
            return Err(super::error::McpToolError::forbidden(
                "Page action tools are disabled",
            ));
        }
        Ok(())
    }

    /// Guard for the highest tier (real input/password content).
    pub async fn require_full_access(&self) -> Result<(), super::error::McpToolError> {
        self.require_act().await?;
        if !self.config.access.full_page_access_enabled {
            return Err(super::error::McpToolError::forbidden(
                "Full page access is disabled",
            ));
        }
        Ok(())
    }

    pub async fn require_environment(
        &self,
        env_uuid: &str,
    ) -> Result<(), super::error::McpToolError> {
        self.require_session().await?;
        if !self.config.access.authorizes_environment(env_uuid) {
            return Err(super::error::McpToolError::forbidden(
                "Environment is not authorized for the HTTP API",
            ));
        }
        self.bridge.get_environment(env_uuid).await?;
        Ok(())
    }
}

pub struct McpServerHandle {
    shutdown_tx: Option<oneshot::Sender<()>>,
    join_handle: JoinHandle<()>,
}

impl McpServerHandle {
    pub fn is_finished(&self) -> bool {
        self.join_handle.is_finished()
    }

    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        let _ = self.join_handle.await;
    }
}

pub async fn spawn_mcp_server(config: McpServerRuntimeConfig) -> Result<McpServerHandle> {
    config.access.validate().map_err(anyhow::Error::msg)?;
    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
    // Bind before returning a handle; another process's /health is not proof
    // that this server started successfully.
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind http api on {addr}"))?;
    let endpoint = config.endpoint();
    let app = build_app(Arc::new(config));
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let join_handle = tokio::spawn(async move {
        log::info!("http api listening on {}", endpoint);

        let server = axum::serve(listener, app).with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        });

        if let Err(error) = server.await {
            log::error!("http api stopped with error: {}", error);
        }
    });

    Ok(McpServerHandle {
        shutdown_tx: Some(shutdown_tx),
        join_handle,
    })
}

pub(crate) fn build_app(state: Arc<McpServerRuntimeConfig>) -> Router {
    let http_api = super::http_api::router(Arc::new(McpServer::new(&state)));

    Router::new()
        .route("/health", get(health_handler))
        .nest_service("/api/v1", http_api)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            security::guard,
        ))
        .with_state(state)
}

async fn health_handler(State(state): State<Arc<McpServerRuntimeConfig>>) -> impl IntoResponse {
    axum::Json(serde_json::json!({
        "status": "running",
        "endpoint": state.endpoint(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    fn config(port: u16) -> McpServerRuntimeConfig {
        let mut access = super::super::config::McpConfig::default();
        access.enabled = true;
        McpServerRuntimeConfig {
            port,
            access,
            read_slots: Arc::new(tokio::sync::Semaphore::new(2)),
        }
    }

    #[tokio::test]
    async fn real_http_transport_rejects_unauthenticated_cross_origin_and_large_requests() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let config = config(port);
        let token = config.access.access_token.clone();
        let app = build_app(Arc::new(config));
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap();
        let url = format!("http://127.0.0.1:{port}");
        let tools = format!("{url}/api/v1/tools");
        assert_eq!(
            client.get(format!("{url}/health")).send().await.unwrap().status(),
            StatusCode::OK
        );
        // 令牌只认 Authorization 头，放在网址里不算
        assert_eq!(
            client.get(format!("{tools}?access_token={token}")).send().await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            client
                .get(&tools)
                .bearer_auth(&token)
                .header("Origin", "https://evil.test")
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            client
                .get(&tools)
                .bearer_auth(&token)
                .header("Host", "evil.test")
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            client
                .post(format!("{tools}/mz_list_authorized_environments"))
                .bearer_auth(&token)
                .body("x".repeat(65537))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::PAYLOAD_TOO_LARGE
        );
        // 默认只有基础档的两个工具
        let listed: serde_json::Value =
            client.get(&tools).bearer_auth(&token).send().await.unwrap().json().await.unwrap();
        let names: Vec<_> = listed["data"]
            .as_array()
            .expect("tool list")
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .collect();
        assert_eq!(names, ["mz_list_authorized_environments", "mz_get_authorized_environment_status"]);
        // MCP 协议的入口已经去掉
        assert_eq!(
            client.post(format!("{url}/mcp")).bearer_auth(&token).body("{}").send().await.unwrap().status(),
            StatusCode::NOT_FOUND
        );
        server.abort();
    }

    #[tokio::test]
    async fn startup_reports_port_conflicts_instead_of_trusting_some_other_health_endpoint() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(spawn_mcp_server(config(port)).await.is_err());
    }
}
