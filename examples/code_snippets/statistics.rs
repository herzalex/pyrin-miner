//! Beispiel-Implementierung für erweiterte Statistiken und Telemetrie
//!
//! Dieses Modul zeigt die vorgeschlagene Implementierung für die
//! umfassende Statistik-Sammlung.
//!
//! # Features
//!
//! - Hashrate-Tracking (aktuell, Durchschnitt, Spitzenwert)
//! - Share-Statistiken (akzeptiert, abgelehnt, stale)
//! - GPU-Telemetrie (Temperatur, Power, Lüfter)
//! - Timing-Statistiken (Uptime, Share-Zeiten)
//! - Prometheus-kompatible Metriken-Export

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Hauptstruktur für Miner-Statistiken
#[derive(Debug)]
pub struct MinerStatistics {
    /// Hashrate-Statistiken
    pub hashrate: HashrateStats,
    /// Share-Statistiken
    pub shares: ShareStats,
    /// Hardware-Statistiken (pro GPU)
    pub hardware: Vec<HardwareStats>,
    /// Timing-Statistiken
    pub timing: TimingStats,
    /// Netzwerk-Statistiken
    pub network: NetworkStats,
}

impl MinerStatistics {
    /// Erstellt neue Statistiken mit der angegebenen GPU-Anzahl
    pub fn new(gpu_count: usize) -> Self {
        Self {
            hashrate: HashrateStats::default(),
            shares: ShareStats::default(),
            hardware: (0..gpu_count).map(|i| HardwareStats::new(i)).collect(),
            timing: TimingStats::new(),
            network: NetworkStats::default(),
        }
    }

    /// Exportiert alle Statistiken als Prometheus-Metriken
    pub fn to_prometheus(&self) -> String {
        let mut output = String::new();
        
        // Hashrate Metriken
        output.push_str(&format!(
            "# HELP pyrin_miner_hashrate_current Current hashrate in H/s\n\
             # TYPE pyrin_miner_hashrate_current gauge\n\
             pyrin_miner_hashrate_current {}\n\n",
            self.hashrate.current.load(Ordering::Relaxed)
        ));
        
        output.push_str(&format!(
            "# HELP pyrin_miner_hashrate_peak Peak hashrate in H/s\n\
             # TYPE pyrin_miner_hashrate_peak gauge\n\
             pyrin_miner_hashrate_peak {}\n\n",
            self.hashrate.peak.load(Ordering::Relaxed)
        ));
        
        // Share Metriken
        output.push_str(&format!(
            "# HELP pyrin_miner_shares_total Total shares submitted\n\
             # TYPE pyrin_miner_shares_total counter\n\
             pyrin_miner_shares_total{{status=\"accepted\"}} {}\n\
             pyrin_miner_shares_total{{status=\"rejected\"}} {}\n\
             pyrin_miner_shares_total{{status=\"stale\"}} {}\n\n",
            self.shares.accepted.load(Ordering::Relaxed),
            self.shares.rejected.load(Ordering::Relaxed),
            self.shares.stale.load(Ordering::Relaxed)
        ));
        
        // GPU Metriken
        for hw in &self.hardware {
            let device_id = hw.device_id;
            output.push_str(&format!(
                "# HELP pyrin_miner_gpu_temperature GPU temperature in Celsius\n\
                 # TYPE pyrin_miner_gpu_temperature gauge\n\
                 pyrin_miner_gpu_temperature{{device=\"{}\"}} {}\n\n",
                device_id,
                hw.temperature.load(Ordering::Relaxed)
            ));
            
            output.push_str(&format!(
                "# HELP pyrin_miner_gpu_power GPU power consumption in Watts\n\
                 # TYPE pyrin_miner_gpu_power gauge\n\
                 pyrin_miner_gpu_power{{device=\"{}\"}} {}\n\n",
                device_id,
                hw.power_usage.load(Ordering::Relaxed)
            ));
            
            output.push_str(&format!(
                "# HELP pyrin_miner_gpu_fan GPU fan speed percentage\n\
                 # TYPE pyrin_miner_gpu_fan gauge\n\
                 pyrin_miner_gpu_fan{{device=\"{}\"}} {}\n\n",
                device_id,
                hw.fan_speed.load(Ordering::Relaxed)
            ));
            
            output.push_str(&format!(
                "# HELP pyrin_miner_gpu_hashrate GPU hashrate in H/s\n\
                 # TYPE pyrin_miner_gpu_hashrate gauge\n\
                 pyrin_miner_gpu_hashrate{{device=\"{}\"}} {}\n\n",
                device_id,
                hw.hashrate.load(Ordering::Relaxed)
            ));
        }
        
        output
    }

    /// Gibt eine formatierte Zusammenfassung aus
    pub fn summary(&self) -> String {
        format!(
            "Hashrate: {:.2} MH/s (Peak: {:.2} MH/s)\n\
             Shares: {} accepted, {} rejected, {} stale ({:.1}% efficiency)\n\
             Uptime: {:?}",
            self.hashrate.current.load(Ordering::Relaxed) as f64 / 1_000_000.0,
            self.hashrate.peak.load(Ordering::Relaxed) as f64 / 1_000_000.0,
            self.shares.accepted.load(Ordering::Relaxed),
            self.shares.rejected.load(Ordering::Relaxed),
            self.shares.stale.load(Ordering::Relaxed),
            self.shares.efficiency() * 100.0,
            self.timing.uptime()
        )
    }
}

/// Hashrate-Statistiken
#[derive(Debug, Default)]
pub struct HashrateStats {
    /// Aktuelle Hashrate (H/s)
    pub current: AtomicU64,
    /// Spitzenwert der Hashrate
    pub peak: AtomicU64,
    /// Historische Hashrate-Werte für Durchschnittsberechnung
    history: RwLock<VecDeque<(Instant, u64)>>,
}

impl HashrateStats {
    /// Aktualisiert die Hashrate und speichert in der Historie
    pub async fn update(&self, hashrate: u64) {
        self.current.store(hashrate, Ordering::Relaxed);
        
        // Peak aktualisieren
        let mut current_peak = self.peak.load(Ordering::Relaxed);
        while hashrate > current_peak {
            match self.peak.compare_exchange_weak(
                current_peak,
                hashrate,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_peak = x,
            }
        }
        
        // Historie aktualisieren
        let mut history = self.history.write().await;
        history.push_back((Instant::now(), hashrate));
        
        // Entferne Einträge älter als 1 Stunde
        let cutoff = Instant::now() - Duration::from_secs(3600);
        while history.front().map(|(t, _)| *t < cutoff).unwrap_or(false) {
            history.pop_front();
        }
    }

    /// Berechnet den Durchschnitt der letzten N Minuten
    pub async fn average(&self, minutes: u64) -> f64 {
        let history = self.history.read().await;
        let cutoff = Instant::now() - Duration::from_secs(minutes * 60);
        
        let values: Vec<u64> = history
            .iter()
            .filter(|(t, _)| *t >= cutoff)
            .map(|(_, v)| *v)
            .collect();
        
        if values.is_empty() {
            return 0.0;
        }
        
        values.iter().sum::<u64>() as f64 / values.len() as f64
    }
}

/// Share-Statistiken
#[derive(Debug, Default)]
pub struct ShareStats {
    /// Anzahl akzeptierter Shares
    pub accepted: AtomicU64,
    /// Anzahl abgelehnter Shares
    pub rejected: AtomicU64,
    /// Anzahl veralteter Shares
    pub stale: AtomicU64,
    /// Anzahl Shares mit niedriger Difficulty
    pub low_diff: AtomicU64,
    /// Anzahl doppelter Shares
    pub duplicate: AtomicU64,
}

impl ShareStats {
    /// Berechnet die Share-Effizienz (akzeptiert / gesamt)
    pub fn efficiency(&self) -> f64 {
        let accepted = self.accepted.load(Ordering::Relaxed);
        let total = accepted
            + self.rejected.load(Ordering::Relaxed)
            + self.stale.load(Ordering::Relaxed);
        
        if total == 0 {
            return 1.0;
        }
        
        accepted as f64 / total as f64
    }

    /// Registriert einen akzeptierten Share
    pub fn record_accepted(&self) {
        self.accepted.fetch_add(1, Ordering::Relaxed);
    }

    /// Registriert einen abgelehnten Share mit Grund
    pub fn record_rejected(&self, reason: RejectReason) {
        match reason {
            RejectReason::Stale => self.stale.fetch_add(1, Ordering::Relaxed),
            RejectReason::LowDifficulty => self.low_diff.fetch_add(1, Ordering::Relaxed),
            RejectReason::Duplicate => self.duplicate.fetch_add(1, Ordering::Relaxed),
            RejectReason::Other => self.rejected.fetch_add(1, Ordering::Relaxed),
        };
    }
}

/// Gründe für Share-Ablehnung
#[derive(Debug, Clone, Copy)]
pub enum RejectReason {
    Stale,
    LowDifficulty,
    Duplicate,
    Other,
}

/// Hardware-Statistiken (pro GPU)
#[derive(Debug)]
pub struct HardwareStats {
    /// GPU-Device-ID
    pub device_id: usize,
    /// Name der GPU
    pub name: RwLock<String>,
    /// Aktuelle Temperatur (Celsius)
    pub temperature: AtomicU64,
    /// Aktueller Stromverbrauch (Watt)
    pub power_usage: AtomicU64,
    /// Aktuelle Lüftergeschwindigkeit (%)
    pub fan_speed: AtomicU64,
    /// GPU-Auslastung (%)
    pub utilization: AtomicU64,
    /// Speicherauslastung (%)
    pub memory_usage: AtomicU64,
    /// GPU-spezifische Hashrate
    pub hashrate: AtomicU64,
}

impl HardwareStats {
    pub fn new(device_id: usize) -> Self {
        Self {
            device_id,
            name: RwLock::new(format!("GPU {}", device_id)),
            temperature: AtomicU64::new(0),
            power_usage: AtomicU64::new(0),
            fan_speed: AtomicU64::new(0),
            utilization: AtomicU64::new(0),
            memory_usage: AtomicU64::new(0),
            hashrate: AtomicU64::new(0),
        }
    }

    /// Aktualisiert alle Hardware-Werte
    pub async fn update(&self, temp: u64, power: u64, fan: u64, util: u64, mem: u64, hash: u64) {
        self.temperature.store(temp, Ordering::Relaxed);
        self.power_usage.store(power, Ordering::Relaxed);
        self.fan_speed.store(fan, Ordering::Relaxed);
        self.utilization.store(util, Ordering::Relaxed);
        self.memory_usage.store(mem, Ordering::Relaxed);
        self.hashrate.store(hash, Ordering::Relaxed);
    }

    /// Prüft ob die Temperatur den Grenzwert überschreitet
    pub fn is_overheating(&self, threshold: u64) -> bool {
        self.temperature.load(Ordering::Relaxed) > threshold
    }
}

/// Timing-Statistiken
#[derive(Debug)]
pub struct TimingStats {
    /// Startzeit des Miners
    pub start_time: Instant,
    /// Letzte Share-Zeit
    pub last_share_time: RwLock<Option<Instant>>,
    /// Durchschnittliche Zeit zwischen Shares
    share_times: RwLock<VecDeque<Duration>>,
}

impl TimingStats {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            last_share_time: RwLock::new(None),
            share_times: RwLock::new(VecDeque::with_capacity(100)),
        }
    }

    /// Gibt die Uptime zurück
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Registriert einen neuen Share für Timing-Berechnung
    pub async fn record_share(&self) {
        let now = Instant::now();
        let mut last_share = self.last_share_time.write().await;
        
        if let Some(last) = *last_share {
            let duration = now - last;
            let mut times = self.share_times.write().await;
            times.push_back(duration);
            
            // Behalte nur die letzten 100 Werte
            while times.len() > 100 {
                times.pop_front();
            }
        }
        
        *last_share = Some(now);
    }

    /// Berechnet die durchschnittliche Zeit zwischen Shares
    pub async fn average_share_time(&self) -> Option<Duration> {
        let times = self.share_times.read().await;
        
        if times.is_empty() {
            return None;
        }
        
        let total: Duration = times.iter().sum();
        Some(total / times.len() as u32)
    }
}

/// Netzwerk-Statistiken
#[derive(Debug, Default)]
pub struct NetworkStats {
    /// Anzahl erfolgreicher Verbindungen
    pub successful_connections: AtomicU64,
    /// Anzahl fehlgeschlagener Verbindungen
    pub failed_connections: AtomicU64,
    /// Anzahl Pool-Wechsel
    pub pool_switches: AtomicU64,
    /// Gesamte empfangene Bytes
    pub bytes_received: AtomicU64,
    /// Gesamte gesendete Bytes
    pub bytes_sent: AtomicU64,
}

/// HTTP-Handler für Prometheus Metriken
/// 
/// HINWEIS: Diese Funktion ist ein Beispiel und erfordert das warp-Crate.
/// Für eine axum-basierte Implementierung siehe web_api.rs
#[cfg(feature = "prometheus-warp")]
pub async fn prometheus_handler(
    stats: Arc<MinerStatistics>,
) -> impl warp::Reply {
    let metrics = stats.to_prometheus();
    warp::reply::with_header(metrics, "Content-Type", "text/plain; charset=utf-8")
}

/// Startet den Prometheus Metrics Server
/// 
/// HINWEIS: Dies ist eine Beispiel-Implementierung.
/// In Produktion sollte der Server mit proper Error-Handling gestartet werden.
#[cfg(feature = "prometheus-warp")]
pub async fn start_prometheus_server(stats: Arc<MinerStatistics>, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    use warp::Filter;
    
    let stats_filter = warp::any().map(move || stats.clone());
    
    let metrics_route = warp::path("metrics")
        .and(warp::get())
        .and(stats_filter)
        .and_then(|stats: Arc<MinerStatistics>| async move {
            Ok::<_, warp::Rejection>(prometheus_handler(stats).await)
        });
    
    log::info!("Starting Prometheus metrics server on port {}", port);
    
    // In Produktion: Nutze tokio::spawn und handle Fehler
    warp::serve(metrics_route).run(([0, 0, 0, 0], port)).await;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_share_efficiency() {
        let stats = ShareStats::default();
        
        // Keine Shares = 100% Effizienz
        assert_eq!(stats.efficiency(), 1.0);
        
        // 8 akzeptiert, 2 abgelehnt = 80% Effizienz
        for _ in 0..8 {
            stats.record_accepted();
        }
        stats.record_rejected(RejectReason::Stale);
        stats.record_rejected(RejectReason::Other);
        
        assert_eq!(stats.efficiency(), 0.8);
    }

    #[test]
    fn test_hardware_overheating() {
        let hw = HardwareStats::new(0);
        
        hw.temperature.store(70, Ordering::Relaxed);
        assert!(!hw.is_overheating(80));
        
        hw.temperature.store(85, Ordering::Relaxed);
        assert!(hw.is_overheating(80));
    }

    #[tokio::test]
    async fn test_hashrate_average() {
        let stats = HashrateStats::default();
        
        // Simuliere Hashrate-Updates
        stats.update(100_000_000).await;
        stats.update(110_000_000).await;
        stats.update(90_000_000).await;
        
        let avg = stats.average(5).await;
        assert!(avg > 0.0);
    }

    #[test]
    fn test_prometheus_output() {
        let stats = MinerStatistics::new(2);
        stats.hashrate.current.store(100_000_000, Ordering::Relaxed);
        stats.shares.accepted.store(100, Ordering::Relaxed);
        
        let prometheus = stats.to_prometheus();
        
        assert!(prometheus.contains("pyrin_miner_hashrate_current"));
        assert!(prometheus.contains("pyrin_miner_shares_total"));
        assert!(prometheus.contains("pyrin_miner_gpu_temperature"));
    }
}
