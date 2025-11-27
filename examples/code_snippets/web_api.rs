//! Beispiel-Implementierung für Web-Dashboard API
//!
//! Dieses Modul zeigt die vorgeschlagene Implementierung für die
//! REST-API und WebSocket-basierte Echtzeit-Updates.
//!
//! # Endpoints
//!
//! - GET /api/v1/status - Aktueller Miner-Status
//! - GET /api/v1/stats - Detaillierte Statistiken
//! - GET /api/v1/workers - Worker-Liste
//! - POST /api/v1/control/pause - Miner pausieren
//! - POST /api/v1/control/resume - Miner fortsetzen
//! - WS /api/v1/ws - WebSocket für Echtzeit-Updates

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, RwLock};

// ============================================================================
// API Responses
// ============================================================================

/// Generische API-Antwort
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub timestamp: u64,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: current_timestamp(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
            timestamp: current_timestamp(),
        }
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ============================================================================
// Status Endpoint
// ============================================================================

/// Miner-Status für /api/v1/status
#[derive(Debug, Serialize)]
pub struct MinerStatus {
    /// Ob der Miner aktiv ist
    pub mining: bool,
    /// Ob der Miner pausiert ist
    pub paused: bool,
    /// Verbundener Pool
    pub pool: Option<String>,
    /// Aktueller Block-Hash (falls verfügbar)
    pub current_block: Option<String>,
    /// Uptime in Sekunden
    pub uptime_seconds: u64,
    /// Version des Miners
    pub version: String,
}

// ============================================================================
// Statistics Endpoint
// ============================================================================

/// Detaillierte Statistiken für /api/v1/stats
#[derive(Debug, Serialize)]
pub struct MinerStats {
    /// Hashrate-Statistiken
    pub hashrate: HashrateInfo,
    /// Share-Statistiken
    pub shares: ShareInfo,
    /// GPU-Statistiken
    pub gpus: Vec<GpuInfo>,
    /// Pool-Statistiken
    pub pool: PoolInfo,
}

#[derive(Debug, Serialize)]
pub struct HashrateInfo {
    /// Aktuelle Hashrate (H/s)
    pub current: f64,
    /// 5-Minuten Durchschnitt
    pub average_5m: f64,
    /// 1-Stunden Durchschnitt
    pub average_1h: f64,
    /// Spitzenwert
    pub peak: f64,
    /// Einheit (H/s, KH/s, MH/s, GH/s)
    pub unit: String,
}

#[derive(Debug, Serialize)]
pub struct ShareInfo {
    /// Akzeptierte Shares
    pub accepted: u64,
    /// Abgelehnte Shares
    pub rejected: u64,
    /// Veraltete Shares
    pub stale: u64,
    /// Effizienz in Prozent
    pub efficiency: f64,
    /// Durchschnittliche Zeit zwischen Shares (Sekunden)
    pub avg_share_time: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct GpuInfo {
    /// GPU-ID
    pub id: u32,
    /// GPU-Name
    pub name: String,
    /// Temperatur (Celsius)
    pub temperature: u32,
    /// Lüftergeschwindigkeit (%)
    pub fan_speed: u32,
    /// Stromverbrauch (Watt)
    pub power: u32,
    /// GPU-Auslastung (%)
    pub utilization: u32,
    /// GPU-spezifische Hashrate
    pub hashrate: f64,
    /// Status
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct PoolInfo {
    /// Pool-URL
    pub url: String,
    /// Verbindungsstatus
    pub connected: bool,
    /// Schwierigkeit
    pub difficulty: f64,
    /// Letzter Share-Zeitpunkt
    pub last_share: Option<u64>,
}

// ============================================================================
// Workers Endpoint
// ============================================================================

/// Worker-Information für /api/v1/workers
#[derive(Debug, Serialize)]
pub struct WorkerInfo {
    /// Worker-ID
    pub id: String,
    /// Worker-Typ (CUDA, OpenCL, CPU)
    pub worker_type: String,
    /// Status (running, paused, error)
    pub status: String,
    /// Hashrate
    pub hashrate: f64,
    /// Akzeptierte Shares
    pub shares_accepted: u64,
    /// Abgelehnte Shares
    pub shares_rejected: u64,
    /// Hardware-Info (für GPUs)
    pub hardware: Option<GpuInfo>,
}

// ============================================================================
// Control Endpoints
// ============================================================================

/// Antwort für Control-Endpoints
#[derive(Debug, Serialize)]
pub struct ControlResponse {
    pub action: String,
    pub success: bool,
    pub message: String,
}

// ============================================================================
// WebSocket Messages
// ============================================================================

/// WebSocket-Nachrichtentypen
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum WsMessage {
    /// Hashrate-Update
    HashrateUpdate(HashrateInfo),
    /// Share gefunden
    ShareFound { accepted: bool, reason: Option<String> },
    /// Neuer Block
    NewBlock { hash: String, timestamp: u64 },
    /// Worker-Status geändert
    WorkerStatus { id: String, status: String },
    /// GPU-Warnung (Temperatur, Fehler)
    GpuWarning { gpu_id: u32, warning_type: String, message: String },
    /// Pool-Event
    PoolEvent { event_type: String, message: String },
    /// Fehler
    Error { code: u32, message: String },
}

// ============================================================================
// API Server Implementation
// ============================================================================

/// API-Server Zustand
pub struct ApiState {
    /// Miner-Status
    pub status: RwLock<MinerStatus>,
    /// Statistiken
    pub stats: RwLock<MinerStats>,
    /// Worker-Liste
    pub workers: RwLock<Vec<WorkerInfo>>,
    /// Ist pausiert?
    pub paused: RwLock<bool>,
    /// WebSocket Broadcast-Sender
    pub ws_sender: broadcast::Sender<WsMessage>,
}

impl ApiState {
    pub fn new() -> Self {
        let (ws_sender, _) = broadcast::channel(100);
        
        Self {
            status: RwLock::new(MinerStatus {
                mining: false,
                paused: false,
                pool: None,
                current_block: None,
                uptime_seconds: 0,
                version: env!("CARGO_PKG_VERSION").to_string(),
            }),
            stats: RwLock::new(MinerStats {
                hashrate: HashrateInfo {
                    current: 0.0,
                    average_5m: 0.0,
                    average_1h: 0.0,
                    peak: 0.0,
                    unit: "H/s".to_string(),
                },
                shares: ShareInfo {
                    accepted: 0,
                    rejected: 0,
                    stale: 0,
                    efficiency: 100.0,
                    avg_share_time: None,
                },
                gpus: Vec::new(),
                pool: PoolInfo {
                    url: String::new(),
                    connected: false,
                    difficulty: 0.0,
                    last_share: None,
                },
            }),
            workers: RwLock::new(Vec::new()),
            paused: RwLock::new(false),
            ws_sender,
        }
    }

    /// Sendet eine WebSocket-Nachricht an alle verbundenen Clients
    pub fn broadcast(&self, message: WsMessage) {
        let _ = self.ws_sender.send(message);
    }
}

/// Beispiel-Implementierung mit axum
#[cfg(feature = "web-api")]
pub mod server {
    use super::*;
    use axum::{
        extract::{State, WebSocketUpgrade},
        http::StatusCode,
        response::IntoResponse,
        routing::{get, post},
        Json, Router,
    };
    use std::net::SocketAddr;

    pub async fn start_server(state: Arc<ApiState>, port: u16) {
        let app = Router::new()
            .route("/api/v1/status", get(get_status))
            .route("/api/v1/stats", get(get_stats))
            .route("/api/v1/workers", get(get_workers))
            .route("/api/v1/control/pause", post(pause_miner))
            .route("/api/v1/control/resume", post(resume_miner))
            .route("/api/v1/ws", get(ws_handler))
            .with_state(state);

        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        log::info!("Starting API server on {}", addr);
        
        // HINWEIS: In Produktion sollte dieser Fehler propagiert werden
        // statt mit unwrap() zu paniken. Beispiel:
        // axum::Server::bind(&addr)
        //     .serve(app.into_make_service())
        //     .await
        //     .map_err(|e| format!("Server error: {}", e))?;
        if let Err(e) = axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
        {
            log::error!("API server error: {}", e);
        }
    }

    async fn get_status(
        State(state): State<Arc<ApiState>>,
    ) -> impl IntoResponse {
        let status = state.status.read().await.clone();
        Json(ApiResponse::success(status))
    }

    async fn get_stats(
        State(state): State<Arc<ApiState>>,
    ) -> impl IntoResponse {
        let stats = state.stats.read().await.clone();
        Json(ApiResponse::success(stats))
    }

    async fn get_workers(
        State(state): State<Arc<ApiState>>,
    ) -> impl IntoResponse {
        let workers = state.workers.read().await.clone();
        Json(ApiResponse::success(workers))
    }

    async fn pause_miner(
        State(state): State<Arc<ApiState>>,
    ) -> impl IntoResponse {
        let mut paused = state.paused.write().await;
        *paused = true;
        
        state.broadcast(WsMessage::PoolEvent {
            event_type: "paused".to_string(),
            message: "Mining paused by user".to_string(),
        });
        
        Json(ApiResponse::success(ControlResponse {
            action: "pause".to_string(),
            success: true,
            message: "Mining paused".to_string(),
        }))
    }

    async fn resume_miner(
        State(state): State<Arc<ApiState>>,
    ) -> impl IntoResponse {
        let mut paused = state.paused.write().await;
        *paused = false;
        
        state.broadcast(WsMessage::PoolEvent {
            event_type: "resumed".to_string(),
            message: "Mining resumed by user".to_string(),
        });
        
        Json(ApiResponse::success(ControlResponse {
            action: "resume".to_string(),
            success: true,
            message: "Mining resumed".to_string(),
        }))
    }

    async fn ws_handler(
        ws: WebSocketUpgrade,
        State(state): State<Arc<ApiState>>,
    ) -> impl IntoResponse {
        ws.on_upgrade(|socket| handle_socket(socket, state))
    }

    async fn handle_socket(
        mut socket: axum::extract::ws::WebSocket,
        state: Arc<ApiState>,
    ) {
        let mut rx = state.ws_sender.subscribe();
        
        loop {
            tokio::select! {
                msg = rx.recv() => {
                    match msg {
                        Ok(ws_msg) => {
                            // Serialisiere mit Error-Handling
                            let json = match serde_json::to_string(&ws_msg) {
                                Ok(j) => j,
                                Err(e) => {
                                    log::error!("Failed to serialize WebSocket message: {}", e);
                                    continue;
                                }
                            };
                            if socket.send(axum::extract::ws::Message::Text(json)).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
        }
    }
}

// ============================================================================
// HTML Dashboard (Minimal)
// ============================================================================

/// Minimales HTML-Dashboard
pub const DASHBOARD_HTML: &str = r#"
<!DOCTYPE html>
<html lang="de">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Pyrin Miner Dashboard</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { 
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #1a1a2e; 
            color: #eee;
            min-height: 100vh;
        }
        .container { max-width: 1200px; margin: 0 auto; padding: 20px; }
        h1 { color: #4cc9f0; margin-bottom: 20px; }
        .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 20px; }
        .card { 
            background: #16213e; 
            border-radius: 10px; 
            padding: 20px;
            border: 1px solid #0f3460;
        }
        .card h2 { color: #4cc9f0; font-size: 14px; margin-bottom: 10px; text-transform: uppercase; }
        .stat { font-size: 32px; font-weight: bold; color: #4cc9f0; }
        .stat-label { font-size: 12px; color: #888; }
        .gpu-row { display: flex; justify-content: space-between; padding: 10px 0; border-bottom: 1px solid #0f3460; }
        .gpu-row:last-child { border-bottom: none; }
        .status-dot { display: inline-block; width: 10px; height: 10px; border-radius: 50%; margin-right: 5px; }
        .status-online { background: #4cc9f0; }
        .status-offline { background: #e94560; }
        .status-paused { background: #ffc107; }
        .btn { 
            background: #4cc9f0; 
            border: none; 
            color: #1a1a2e; 
            padding: 10px 20px; 
            border-radius: 5px;
            cursor: pointer;
            font-weight: bold;
        }
        .btn:hover { background: #3ab4d8; }
        .btn-pause { background: #ffc107; }
        .btn-resume { background: #28a745; color: white; }
    </style>
</head>
<body>
    <div class="container">
        <h1>⛏️ Pyrin Miner Dashboard</h1>
        
        <div class="grid">
            <div class="card">
                <h2>Hashrate</h2>
                <div class="stat" id="hashrate">-- MH/s</div>
                <div class="stat-label">Peak: <span id="hashrate-peak">-- MH/s</span></div>
            </div>
            
            <div class="card">
                <h2>Shares</h2>
                <div class="stat" id="shares-accepted">0</div>
                <div class="stat-label">
                    Rejected: <span id="shares-rejected">0</span> |
                    Efficiency: <span id="efficiency">100%</span>
                </div>
            </div>
            
            <div class="card">
                <h2>Status</h2>
                <div>
                    <span class="status-dot status-online" id="status-dot"></span>
                    <span id="status-text">Connecting...</span>
                </div>
                <div class="stat-label">Pool: <span id="pool">-</span></div>
            </div>
            
            <div class="card">
                <h2>Uptime</h2>
                <div class="stat" id="uptime">00:00:00</div>
            </div>
        </div>
        
        <div class="card" style="margin-top: 20px;">
            <h2>GPUs</h2>
            <div id="gpu-list">Loading...</div>
        </div>
        
        <div class="card" style="margin-top: 20px;">
            <h2>Control</h2>
            <button class="btn btn-pause" id="btn-pause" onclick="toggleMining()">Pause Mining</button>
        </div>
    </div>
    
    <script>
        let ws;
        let isPaused = false;
        
        function connect() {
            ws = new WebSocket(`ws://${window.location.host}/api/v1/ws`);
            
            ws.onopen = () => {
                document.getElementById('status-text').textContent = 'Connected';
                document.getElementById('status-dot').className = 'status-dot status-online';
            };
            
            ws.onclose = () => {
                document.getElementById('status-text').textContent = 'Disconnected';
                document.getElementById('status-dot').className = 'status-dot status-offline';
                setTimeout(connect, 5000);
            };
            
            ws.onmessage = (event) => {
                const msg = JSON.parse(event.data);
                handleMessage(msg);
            };
        }
        
        function handleMessage(msg) {
            switch(msg.type) {
                case 'HashrateUpdate':
                    document.getElementById('hashrate').textContent = 
                        (msg.data.current / 1000000).toFixed(2) + ' MH/s';
                    document.getElementById('hashrate-peak').textContent = 
                        (msg.data.peak / 1000000).toFixed(2) + ' MH/s';
                    break;
                case 'ShareFound':
                    if (msg.data.accepted) {
                        const el = document.getElementById('shares-accepted');
                        el.textContent = parseInt(el.textContent) + 1;
                    } else {
                        const el = document.getElementById('shares-rejected');
                        el.textContent = parseInt(el.textContent) + 1;
                    }
                    break;
            }
        }
        
        async function toggleMining() {
            const endpoint = isPaused ? '/api/v1/control/resume' : '/api/v1/control/pause';
            await fetch(endpoint, { method: 'POST' });
            isPaused = !isPaused;
            
            const btn = document.getElementById('btn-pause');
            btn.textContent = isPaused ? 'Resume Mining' : 'Pause Mining';
            btn.className = isPaused ? 'btn btn-resume' : 'btn btn-pause';
        }
        
        async function loadStats() {
            try {
                const res = await fetch('/api/v1/stats');
                const data = await res.json();
                if (data.success) {
                    updateStats(data.data);
                }
            } catch (e) {
                console.error('Failed to load stats:', e);
            }
        }
        
        function updateStats(stats) {
            document.getElementById('hashrate').textContent = 
                (stats.hashrate.current / 1000000).toFixed(2) + ' MH/s';
            document.getElementById('shares-accepted').textContent = stats.shares.accepted;
            document.getElementById('shares-rejected').textContent = stats.shares.rejected;
            document.getElementById('efficiency').textContent = stats.shares.efficiency.toFixed(1) + '%';
            document.getElementById('pool').textContent = stats.pool.url || '-';
            
            const gpuHtml = stats.gpus.map(gpu => `
                <div class="gpu-row">
                    <span>${gpu.name}</span>
                    <span>${gpu.temperature}°C | ${gpu.power}W | ${(gpu.hashrate/1000000).toFixed(2)} MH/s</span>
                </div>
            `).join('');
            document.getElementById('gpu-list').innerHTML = gpuHtml || 'No GPUs detected';
        }
        
        connect();
        loadStats();
        setInterval(loadStats, 5000);
    </script>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_success() {
        let response = ApiResponse::success("test data");
        assert!(response.success);
        assert_eq!(response.data, Some("test data"));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_response_error() {
        let response: ApiResponse<()> = ApiResponse::error("Something went wrong");
        assert!(!response.success);
        assert!(response.data.is_none());
        assert_eq!(response.error, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_api_state_creation() {
        let state = ApiState::new();
        assert!(!*futures::executor::block_on(state.paused.read()));
    }
}
