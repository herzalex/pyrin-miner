//! Beispiel-Implementierung für Konfigurationsdatei-Support
//!
//! Dieses Modul zeigt die vorgeschlagene Implementierung für die
//! TOML-basierte Konfigurationsdatei-Unterstützung.
//!
//! # Verwendung
//!
//! ```rust
//! use config_loader::MinerConfig;
//!
//! let config = MinerConfig::load("config.toml")?;
//! // oder mit CLI-Argumenten kombiniert:
//! let config = MinerConfig::load_with_overrides("config.toml", &cli_args)?;
//! ```

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Hauptkonfiguration für den Miner
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerConfig {
    /// Grundlegende Mining-Einstellungen
    pub miner: MinerSettings,
    /// Netzwerk-Einstellungen
    pub network: NetworkSettings,
    /// Pool-Konfiguration
    #[serde(default)]
    pub pools: PoolsSettings,
    /// GPU-Einstellungen
    #[serde(default)]
    pub gpu: GpuSettings,
    /// Monitoring-Einstellungen
    #[serde(default)]
    pub monitoring: MonitoringSettings,
    /// Benachrichtigungs-Einstellungen
    #[serde(default)]
    pub notifications: NotificationSettings,
    /// Logging-Einstellungen
    #[serde(default)]
    pub logging: LoggingSettings,
    /// Erweiterte Einstellungen
    #[serde(default)]
    pub advanced: AdvancedSettings,
}

impl MinerConfig {
    /// Lädt die Konfiguration aus einer TOML-Datei
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| ConfigError::FileRead(path.as_ref().to_path_buf(), e))?;
        
        toml::from_str(&content)
            .map_err(|e| ConfigError::Parse(e.to_string()))
    }

    /// Lädt die Konfiguration und überschreibt mit CLI-Argumenten
    pub fn load_with_cli_overrides<P: AsRef<Path>>(
        path: P,
        cli: &CliOverrides,
    ) -> Result<Self, ConfigError> {
        let mut config = Self::load(path)?;
        
        // CLI-Argumente überschreiben Konfigurationsdatei
        if let Some(ref address) = cli.mining_address {
            config.miner.address = address.clone();
        }
        if let Some(threads) = cli.threads {
            config.miner.threads = threads;
        }
        if let Some(ref node) = cli.node_address {
            config.network.node_address = node.clone();
        }
        if let Some(port) = cli.port {
            config.network.port = port;
        }
        
        config.validate()?;
        Ok(config)
    }

    /// Validiert die Konfiguration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validiere Mining-Adresse
        if self.miner.address.is_empty() || !self.miner.address.starts_with("pyrin:") {
            return Err(ConfigError::Validation(
                "Mining address must start with 'pyrin:'".to_string(),
            ));
        }

        // Validiere Pools wenn Stratum verwendet wird
        if self.network.protocol == Protocol::Stratum && self.pools.servers.is_empty() {
            return Err(ConfigError::Validation(
                "At least one pool server is required for Stratum protocol".to_string(),
            ));
        }

        // Validiere GPU-Einstellungen
        if self.gpu.cuda.enabled && self.gpu.opencl.enabled {
            log::warn!("Both CUDA and OpenCL are enabled. This may cause issues.");
        }

        Ok(())
    }

    /// Speichert die Konfiguration in eine TOML-Datei
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| ConfigError::Serialize(e.to_string()))?;
        
        std::fs::write(&path, content)
            .map_err(|e| ConfigError::FileWrite(path.as_ref().to_path_buf(), e))?;
        
        Ok(())
    }

    /// Erstellt eine Standardkonfiguration
    pub fn default_config(address: String) -> Self {
        Self {
            miner: MinerSettings {
                address,
                threads: 0,
                mine_when_not_synced: false,
            },
            network: NetworkSettings::default(),
            pools: PoolsSettings::default(),
            gpu: GpuSettings::default(),
            monitoring: MonitoringSettings::default(),
            notifications: NotificationSettings::default(),
            logging: LoggingSettings::default(),
            advanced: AdvancedSettings::default(),
        }
    }
}

/// Mining-Grundeinstellungen
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerSettings {
    /// Wallet-Adresse für Mining-Belohnungen
    pub address: String,
    /// Anzahl der CPU-Threads (0 = nur GPU)
    #[serde(default)]
    pub threads: u32,
    /// Mining auch wenn Node nicht synchronisiert ist
    #[serde(default)]
    pub mine_when_not_synced: bool,
}

/// Netzwerk-Einstellungen
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSettings {
    /// Node-Adresse
    #[serde(default = "default_node_address")]
    pub node_address: String,
    /// Node-Port
    #[serde(default = "default_port")]
    pub port: u16,
    /// Protokoll
    #[serde(default)]
    pub protocol: Protocol,
    /// Testnet aktivieren
    #[serde(default)]
    pub testnet: bool,
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            node_address: default_node_address(),
            port: default_port(),
            protocol: Protocol::Grpc,
            testnet: false,
        }
    }
}

fn default_node_address() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    13110
}

/// Protokoll-Typ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    #[default]
    Grpc,
    Stratum,
    StratumV2,
}

/// Pool-Konfiguration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PoolsSettings {
    /// Failover-Timeout in Sekunden
    #[serde(default = "default_failover_timeout")]
    pub failover_timeout: u64,
    /// Maximale Wiederholungsversuche
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// Pool-Server
    #[serde(default)]
    pub servers: Vec<PoolServerConfig>,
}

fn default_failover_timeout() -> u64 {
    30
}

fn default_max_retries() -> u32 {
    5
}

/// Einzelne Pool-Server-Konfiguration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolServerConfig {
    /// Server-Adresse
    pub address: String,
    /// Server-Port
    pub port: u16,
    /// Gewichtung (höher = mehr Priorität)
    #[serde(default = "default_weight")]
    pub weight: u32,
}

fn default_weight() -> u32 {
    100
}

/// GPU-Einstellungen
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GpuSettings {
    /// CUDA-Einstellungen
    #[serde(default)]
    pub cuda: CudaSettings,
    /// OpenCL-Einstellungen
    #[serde(default)]
    pub opencl: OpenClSettings,
}

/// CUDA-spezifische Einstellungen
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CudaSettings {
    /// CUDA aktivieren
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Spezifische GPU-IDs (leer = alle)
    #[serde(default)]
    pub devices: Vec<u32>,
    /// Workload pro GPU
    #[serde(default)]
    pub workload: Vec<f32>,
    /// Workload ist absolut
    #[serde(default)]
    pub workload_absolute: bool,
    /// Blocking Sync deaktivieren
    #[serde(default)]
    pub no_blocking_sync: bool,
    /// Overclock-Einstellungen
    #[serde(default)]
    pub overclock: OverclockSettings,
}

impl Default for CudaSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            devices: Vec::new(),
            workload: Vec::new(),
            workload_absolute: false,
            no_blocking_sync: false,
            overclock: OverclockSettings::default(),
        }
    }
}

/// OpenCL-spezifische Einstellungen
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenClSettings {
    /// OpenCL aktivieren
    #[serde(default)]
    pub enabled: bool,
    /// Plattform-ID
    #[serde(default)]
    pub platform: Option<u32>,
    /// Spezifische GPU-IDs
    #[serde(default)]
    pub devices: Vec<u32>,
    /// Workload
    #[serde(default)]
    pub workload: Vec<f32>,
    /// AMD GPUs deaktivieren
    #[serde(default)]
    pub amd_disable: bool,
    /// Experimentelle AMD Features
    #[serde(default)]
    pub experimental_amd: bool,
}

/// Overclock-Einstellungen
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OverclockSettings {
    /// Core Clock Lock (MHz)
    #[serde(default)]
    pub core_clocks: Vec<u32>,
    /// Memory Clock Lock (MHz)
    #[serde(default)]
    pub mem_clocks: Vec<u32>,
    /// Power Limit (Watt)
    #[serde(default)]
    pub power_limits: Vec<u32>,
}

/// Monitoring-Einstellungen
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringSettings {
    /// Web-Dashboard aktivieren
    #[serde(default)]
    pub web_dashboard: bool,
    /// Web-Server Port
    #[serde(default = "default_web_port")]
    pub web_port: u16,
    /// Web-Server Bind-Adresse
    #[serde(default = "default_web_bind")]
    pub web_bind: String,
    /// API aktivieren
    #[serde(default)]
    pub api_enabled: bool,
    /// API-Schlüssel
    #[serde(default)]
    pub api_key: Option<String>,
    /// Prometheus aktivieren
    #[serde(default)]
    pub prometheus_enabled: bool,
    /// Prometheus Port
    #[serde(default = "default_prometheus_port")]
    pub prometheus_port: u16,
}

impl Default for MonitoringSettings {
    fn default() -> Self {
        Self {
            web_dashboard: false,
            web_port: default_web_port(),
            web_bind: default_web_bind(),
            api_enabled: false,
            api_key: None,
            prometheus_enabled: false,
            prometheus_port: default_prometheus_port(),
        }
    }
}

fn default_web_port() -> u16 {
    8080
}

fn default_web_bind() -> String {
    "0.0.0.0".to_string()
}

fn default_prometheus_port() -> u16 {
    9090
}

fn default_true() -> bool {
    true
}

/// Benachrichtigungs-Einstellungen
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NotificationSettings {
    /// Benachrichtigung bei Block-Fund
    #[serde(default)]
    pub on_block_found: bool,
    /// Benachrichtigung bei Verbindungsproblemen
    #[serde(default)]
    pub on_connection_lost: bool,
    /// Benachrichtigung bei GPU-Fehler
    #[serde(default)]
    pub on_gpu_error: bool,
    /// Temperatur-Warnung
    #[serde(default)]
    pub temperature_warning: bool,
    /// Temperatur-Schwellwert
    #[serde(default = "default_temp_threshold")]
    pub temperature_threshold: u32,
    /// Desktop-Benachrichtigungen
    #[serde(default)]
    pub desktop_notifications: bool,
    /// Webhook-URL
    #[serde(default)]
    pub webhook_url: Option<String>,
}

fn default_temp_threshold() -> u32 {
    80
}

/// Logging-Einstellungen
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingSettings {
    /// Log-Level
    #[serde(default = "default_log_level")]
    pub level: String,
    /// Log-Datei
    #[serde(default)]
    pub file: Option<PathBuf>,
    /// Max Log-Größe in MB
    #[serde(default)]
    pub max_size_mb: u32,
    /// Max Anzahl Log-Dateien
    #[serde(default = "default_max_files")]
    pub max_files: u32,
    /// Farbige Ausgabe
    #[serde(default = "default_true")]
    pub colored: bool,
}

impl Default for LoggingSettings {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            file: None,
            max_size_mb: 100,
            max_files: 5,
            colored: true,
        }
    }
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_max_files() -> u32 {
    5
}

/// Erweiterte Einstellungen
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedSettings {
    /// Nonce-Generator Typ
    #[serde(default = "default_nonce_gen")]
    pub nonce_gen: String,
    /// Adaptive Workload
    #[serde(default)]
    pub adaptive_workload: bool,
    /// Cooldown-Periode in Sekunden
    #[serde(default = "default_cooldown")]
    pub cooldown_period: u64,
    /// Max GPU-Temperatur
    #[serde(default = "default_max_temp")]
    pub max_gpu_temperature: u32,
    /// Automatischer Neustart
    #[serde(default)]
    pub auto_restart: bool,
    /// Neustart-Verzögerung in Sekunden
    #[serde(default = "default_restart_delay")]
    pub restart_delay: u64,
}

impl Default for AdvancedSettings {
    fn default() -> Self {
        Self {
            nonce_gen: default_nonce_gen(),
            adaptive_workload: false,
            cooldown_period: 30,
            max_gpu_temperature: 85,
            auto_restart: false,
            restart_delay: 10,
        }
    }
}

fn default_nonce_gen() -> String {
    "lean".to_string()
}

fn default_cooldown() -> u64 {
    30
}

fn default_max_temp() -> u32 {
    85
}

fn default_restart_delay() -> u64 {
    10
}

/// CLI-Überschreibungen für Konfigurationsoptionen
#[derive(Debug, Default)]
pub struct CliOverrides {
    pub mining_address: Option<String>,
    pub threads: Option<u32>,
    pub node_address: Option<String>,
    pub port: Option<u16>,
}

/// Konfigurationsfehler
#[derive(Debug)]
pub enum ConfigError {
    FileRead(PathBuf, std::io::Error),
    FileWrite(PathBuf, std::io::Error),
    Parse(String),
    Serialize(String),
    Validation(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileRead(path, e) => write!(f, "Failed to read config file {:?}: {}", path, e),
            Self::FileWrite(path, e) => write!(f, "Failed to write config file {:?}: {}", path, e),
            Self::Parse(e) => write!(f, "Failed to parse config: {}", e),
            Self::Serialize(e) => write!(f, "Failed to serialize config: {}", e),
            Self::Validation(e) => write!(f, "Config validation failed: {}", e),
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MinerConfig::default_config("pyrin:qztest123".to_string());
        
        assert_eq!(config.miner.address, "pyrin:qztest123");
        assert_eq!(config.miner.threads, 0);
        assert_eq!(config.network.port, 13110);
        assert!(config.gpu.cuda.enabled);
    }

    #[test]
    fn test_validation() {
        let mut config = MinerConfig::default_config("pyrin:qztest123".to_string());
        assert!(config.validate().is_ok());

        config.miner.address = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_serialization() {
        let config = MinerConfig::default_config("pyrin:qztest123".to_string());
        let toml = toml::to_string_pretty(&config).unwrap();
        
        assert!(toml.contains("pyrin:qztest123"));
        assert!(toml.contains("[miner]"));
        assert!(toml.contains("[network]"));
    }

    #[test]
    fn test_deserialization() {
        let toml = r#"
            [miner]
            address = "pyrin:qztest123"
            threads = 4

            [network]
            node_address = "192.168.1.100"
            port = 13110
        "#;

        let config: MinerConfig = toml::from_str(toml).unwrap();
        
        assert_eq!(config.miner.address, "pyrin:qztest123");
        assert_eq!(config.miner.threads, 4);
        assert_eq!(config.network.node_address, "192.168.1.100");
    }
}
