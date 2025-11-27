# Pyrin-Miner Code-Beispiele

Dieses Verzeichnis enthält Referenz-Implementierungen für die vorgeschlagenen Features im Verbesserungsplan.

## Übersicht

| Datei | Feature | Priorität | Aufwand |
|-------|---------|-----------|---------|
| `pool_failover.rs` | Multi-Pool Failover | 🔴 Hoch | 1-2 Wochen |
| `statistics.rs` | Erweiterte Statistiken & Prometheus | 🔴 Hoch | 1-2 Wochen |
| `config_loader.rs` | TOML Konfigurationsdatei | 🟡 Mittel | 1 Woche |
| `web_api.rs` | Web-Dashboard & REST API | 🔴 Hoch | 2-3 Wochen |

## Verwendung der Beispiele

Diese Dateien sind **Referenz-Implementierungen** - sie zeigen die vorgeschlagene Architektur und API-Design. Sie können direkt in das Projekt integriert oder als Vorlage verwendet werden.

### Pool Failover (`pool_failover.rs`)

```rust
use pool_manager::{PoolManager, PoolConfig, PoolServer};

let config = PoolConfig {
    servers: vec![
        PoolServer::new("pool1.pyrin.network", 3333, 100),
        PoolServer::new("pool2.pyrin.network", 3333, 50),
    ],
    failover_timeout: Duration::from_secs(30),
    ..Default::default()
};

let manager = Arc::new(PoolManager::new(config));

// Starte Hintergrund-Gesundheitsprüfung
manager.clone().start_health_check_task();

// Verbinde mit bestem Pool
manager.connect().await?;
```

**Features:**
- Automatischer Wechsel bei Pool-Ausfall
- Gewichtete Pool-Priorisierung
- Hintergrund-Gesundheitsprüfung
- Thread-safe Status-Tracking

### Erweiterte Statistiken (`statistics.rs`)

```rust
use statistics::{MinerStatistics, RejectReason};

let stats = MinerStatistics::new(2); // 2 GPUs

// Hashrate aktualisieren
stats.hashrate.update(100_000_000).await; // 100 MH/s

// Share registrieren
stats.shares.record_accepted();
stats.timing.record_share().await;

// Prometheus-Metriken exportieren
let metrics = stats.to_prometheus();
```

**Features:**
- Hashrate-Tracking (aktuell, Durchschnitt, Peak)
- Share-Statistiken mit Effizienz-Berechnung
- GPU-Telemetrie (Temperatur, Power, Lüfter)
- Prometheus-kompatibler Export
- Timing-Statistiken

### Konfigurationsdatei (`config_loader.rs`)

```rust
use config_loader::{MinerConfig, CliOverrides};

// Lade Konfiguration aus Datei
let config = MinerConfig::load("config.toml")?;

// Oder mit CLI-Überschreibungen
let cli = CliOverrides {
    mining_address: Some("pyrin:qz...".to_string()),
    ..Default::default()
};
let config = MinerConfig::load_with_cli_overrides("config.toml", &cli)?;

// Validierung
config.validate()?;
```

**Features:**
- TOML-basierte Konfiguration
- CLI-Argument-Überschreibungen
- Vollständige Validierung
- Serialisierung/Deserialisierung

### Web-Dashboard & API (`web_api.rs`)

```rust
use web_api::{ApiState, server};

let state = Arc::new(ApiState::new());

// Starte API-Server
tokio::spawn(server::start_server(state.clone(), 8080));

// Sende WebSocket-Nachricht an alle Clients
state.broadcast(WsMessage::HashrateUpdate(hashrate_info));
```

**Endpoints:**
- `GET /api/v1/status` - Miner-Status
- `GET /api/v1/stats` - Detaillierte Statistiken
- `GET /api/v1/workers` - Worker-Liste
- `POST /api/v1/control/pause` - Mining pausieren
- `POST /api/v1/control/resume` - Mining fortsetzen
- `WS /api/v1/ws` - WebSocket für Echtzeit-Updates

**Features:**
- RESTful API mit JSON-Antworten
- WebSocket für Live-Updates
- Minimales HTML-Dashboard
- Broadcast-Unterstützung

## Abhängigkeiten

Um diese Beispiele zu verwenden, fügen Sie folgende Abhängigkeiten zu `Cargo.toml` hinzu:

```toml
[dependencies]
# Für alle Beispiele
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
log = "0.4"

# Für config_loader.rs
toml = "0.8"

# Für web_api.rs (optional)
axum = "0.7"
warp = "0.3"  # Alternative zu axum

# Für statistics.rs mit Prometheus
# prometheus = "0.13"
```

## Integration in das Projekt

### Schritt 1: Modul hinzufügen

```rust
// src/lib.rs oder src/main.rs
mod pool_manager;
mod statistics;
mod config;
mod web_api;
```

### Schritt 2: Feature-Flags (optional)

```toml
[features]
default = []
web-dashboard = ["axum", "tokio-tungstenite"]
prometheus = ["prometheus-client"]
config-file = ["toml"]
```

### Schritt 3: Tests ausführen

```bash
# Einzelnes Modul testen
cargo test --package pyrin-miner -- pool_manager

# Alle Statistik-Tests
cargo test --package pyrin-miner -- statistics
```

## Architektur-Diagramm

```
┌─────────────────────────────────────────────────────────────┐
│                     Pyrin-Miner                              │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │ Config       │  │ Pool         │  │ Web          │       │
│  │ Loader       │  │ Manager      │  │ Dashboard    │       │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘       │
│         │                 │                 │               │
│         ▼                 ▼                 ▼               │
│  ┌─────────────────────────────────────────────────────┐    │
│  │                  MinerManager                        │    │
│  │  ┌─────────────────────────────────────────────┐   │    │
│  │  │              Statistics                       │   │    │
│  │  │  - HashrateStats                             │   │    │
│  │  │  - ShareStats                                │   │    │
│  │  │  - HardwareStats                             │   │    │
│  │  └─────────────────────────────────────────────┘   │    │
│  └─────────────────────────────────────────────────────┘    │
│         │                                                    │
│         ▼                                                    │
│  ┌─────────────────────────────────────────────────────┐    │
│  │                    Workers                           │    │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐          │    │
│  │  │ CUDA     │  │ OpenCL   │  │ CPU      │          │    │
│  │  │ Worker   │  │ Worker   │  │ Worker   │          │    │
│  │  └──────────┘  └──────────┘  └──────────┘          │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Weitere Informationen

Siehe `IMPROVEMENTS.md` für den vollständigen Verbesserungsplan und die Roadmap.
