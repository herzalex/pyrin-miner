# Pyrin-Miner Beispiele

Dieses Verzeichnis enthält Beispiele und Vorlagen für verschiedene Verwendungszwecke.

## Inhalt

### 📋 Konfigurationsdatei

**`config_example.toml`** - Vollständige Beispiel-Konfigurationsdatei mit allen verfügbaren Optionen.

Die Konfigurationsdatei unterstützt (oder wird in Zukunft unterstützen):
- Mining-Grundeinstellungen (Adresse, Threads)
- Netzwerk-Konfiguration (Node, Port, Protokoll)
- Multi-Pool-Setup mit Failover
- CUDA GPU-Einstellungen mit Overclocking
- OpenCL GPU-Einstellungen
- Monitoring und Dashboard
- Benachrichtigungen
- Logging
- Profitabilitäts-Berechnung

### 🐳 Docker

**`docker/Dockerfile`** - Standard-Dockerfile für OpenCL-Mining

**`docker/Dockerfile.cuda`** - Dockerfile mit NVIDIA CUDA-Unterstützung

**`docker/docker-compose.yml`** - Docker Compose Konfiguration für einfaches Deployment

## Verwendung

### Konfigurationsdatei

1. Kopiere `config_example.toml` nach `config.toml`
2. Passe die Werte an (insbesondere `address`)
3. Starte den Miner:
   ```bash
   # Zukünftig (wenn Config-File-Support implementiert ist):
   pyrin-miner --config config.toml
   
   # Aktuell:
   pyrin-miner --mining-address pyrin:qz... [weitere Optionen]
   ```

### Docker

1. Baue das Docker-Image:
   ```bash
   # Standard (OpenCL)
   docker build -t pyrin-miner:latest -f examples/docker/Dockerfile .
   
   # Mit CUDA
   docker build -t pyrin-miner:cuda -f examples/docker/Dockerfile.cuda .
   ```

2. Starte mit Docker:
   ```bash
   # Einzelner Container
   docker run --gpus all pyrin-miner:cuda \
     --mining-address pyrin:qzYOUR_ADDRESS_HERE \
     --pyrin-address 192.168.1.100
   
   # Mit Docker Compose
   cd examples/docker
   export MINING_ADDRESS=pyrin:qzYOUR_ADDRESS_HERE
   docker-compose up -d
   ```

3. Logs überprüfen:
   ```bash
   docker logs -f pyrin-miner
   ```

4. Container stoppen:
   ```bash
   docker-compose down
   ```

## GPU-Passthrough

### NVIDIA (CUDA)

Voraussetzungen:
- NVIDIA Driver installiert
- NVIDIA Container Toolkit installiert
- nvidia-docker2 Package

Installation (Ubuntu/Debian):
```bash
# NVIDIA Container Toolkit hinzufügen
distribution=$(. /etc/os-release;echo $ID$VERSION_ID)
curl -s -L https://nvidia.github.io/nvidia-docker/gpgkey | sudo apt-key add -
curl -s -L https://nvidia.github.io/nvidia-docker/$distribution/nvidia-docker.list | \
  sudo tee /etc/apt/sources.list.d/nvidia-docker.list

# Installieren
sudo apt-get update
sudo apt-get install -y nvidia-docker2

# Docker Daemon neustarten
sudo systemctl restart docker

# Testen
docker run --rm --gpus all nvidia/cuda:11.8.0-base-ubuntu22.04 nvidia-smi
```

### AMD (OpenCL)

Für AMD-GPUs muss das `amdgpu`-Gerät durchgereicht werden:
```bash
docker run --device=/dev/dri --device=/dev/kfd \
  --group-add video \
  pyrin-miner:latest [optionen]
```

## Troubleshooting

### GPU wird nicht erkannt

1. Überprüfe nvidia-smi funktioniert:
   ```bash
   nvidia-smi
   ```

2. Überprüfe Docker GPU-Zugang:
   ```bash
   docker run --rm --gpus all nvidia/cuda:11.8.0-base-ubuntu22.04 nvidia-smi
   ```

3. Überprüfe NVIDIA Container Runtime:
   ```bash
   docker info | grep -i runtime
   ```

### Performance-Probleme

1. Erhöhe den Workload:
   ```bash
   --cuda-workload 128
   ```

2. Deaktiviere Blocking Sync für mehr Performance (höhere CPU-Nutzung):
   ```bash
   --cuda-no-blocking-sync
   ```

3. Überprüfe GPU-Temperatur - bei Überhitzung wird die Leistung gedrosselt

### Verbindungsprobleme

1. Stelle sicher, dass der Node erreichbar ist:
   ```bash
   curl http://NODE_IP:13110
   ```

2. Bei Docker: Nutze `--network host` oder stelle sicher, dass die Ports erreichbar sind

## Support

Bei Fragen oder Problemen:
1. Überprüfe die Dokumentation in `IMPROVEMENTS.md`
2. Öffne ein Issue im Repository
3. Besuche den Discord-Server
