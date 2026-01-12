# Self-Hosted Runner Setup Guide

This guide explains how to set up self-hosted GitHub Actions runners for cross-platform E2E testing.

## Overview

For comprehensive E2E testing, we need runners on:
- Linux (Ubuntu 22.04+ recommended)
- macOS (14+ recommended)
- Windows (11 recommended)

## Prerequisites

### All Platforms
- GitHub account with repository access
- Network access to GitHub Actions
- At least 4GB RAM, 2 CPU cores
- 20GB free disk space

### Linux
```bash
# Install system dependencies
sudo apt-get update
sudo apt-get install -y \
    libasound2-dev \
    libssl-dev \
    pkg-config \
    pipewire \
    pipewire-pulse \
    wireplumber

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify PipeWire
systemctl --user status pipewire
```

### macOS
```bash
# Install Homebrew if not present
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install dependencies
brew install blackhole-2ch switchaudio-osx

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify BlackHole
system_profiler SPAudioDataType | grep BlackHole
```

### Windows
```powershell
# Install VB-Audio Virtual Cable
# Download from: https://vb-audio.com/Cable/
# Run the installer and restart

# Install Rust (via rustup-init.exe)
# Download from: https://rustup.rs/

# Install Visual Studio Build Tools
# Download from: https://visualstudio.microsoft.com/visual-cpp-build-tools/
```

## GitHub Runner Installation

### 1. Get Runner Token

1. Go to repository Settings → Actions → Runners
2. Click "New self-hosted runner"
3. Copy the token (valid for 1 hour)

### 2. Install Runner

#### Linux
```bash
# Create runner directory
mkdir -p ~/actions-runner && cd ~/actions-runner

# Download runner
curl -o actions-runner-linux-x64-2.311.0.tar.gz -L \
  https://github.com/actions/runner/releases/download/v2.311.0/actions-runner-linux-x64-2.311.0.tar.gz

# Extract
tar xzf ./actions-runner-linux-x64-2.311.0.tar.gz

# Configure
./config.sh --url https://github.com/koedame/p2paudiosession \
            --token YOUR_TOKEN \
            --labels linux,e2e

# Install as service
sudo ./svc.sh install
sudo ./svc.sh start
```

#### macOS
```bash
# Create runner directory
mkdir -p ~/actions-runner && cd ~/actions-runner

# Download runner
curl -o actions-runner-osx-x64-2.311.0.tar.gz -L \
  https://github.com/actions/runner/releases/download/v2.311.0/actions-runner-osx-x64-2.311.0.tar.gz

# Extract
tar xzf ./actions-runner-osx-x64-2.311.0.tar.gz

# Configure
./config.sh --url https://github.com/koedame/p2paudiosession \
            --token YOUR_TOKEN \
            --labels macos,e2e

# Install as service
./svc.sh install
./svc.sh start
```

#### Windows
```powershell
# Create runner directory
mkdir C:\actions-runner ; cd C:\actions-runner

# Download runner
Invoke-WebRequest -Uri https://github.com/actions/runner/releases/download/v2.311.0/actions-runner-win-x64-2.311.0.zip -OutFile actions-runner-win-x64-2.311.0.zip

# Extract
Add-Type -AssemblyName System.IO.Compression.FileSystem
[System.IO.Compression.ZipFile]::ExtractToDirectory("$PWD\actions-runner-win-x64-2.311.0.zip", "$PWD")

# Configure
.\config.cmd --url https://github.com/koedame/p2paudiosession `
             --token YOUR_TOKEN `
             --labels windows,e2e

# Install as service
.\svc.cmd install
.\svc.cmd start
```

## Virtual Audio Setup

### Linux (PipeWire)

Create virtual devices:
```bash
./tests/e2e/scripts/setup-virtual-audio-linux.sh create
```

Verify:
```bash
./tests/e2e/scripts/setup-virtual-audio-linux.sh status
```

### macOS (BlackHole)

Verify installation:
```bash
./tests/e2e/scripts/setup-virtual-audio-macos.sh check
```

Set default device:
```bash
./tests/e2e/scripts/setup-virtual-audio-macos.sh default
```

### Windows (VB-Cable)

Check installation:
```powershell
.\tests\e2e\scripts\setup-virtual-audio-windows.ps1 -Action check
```

## Workflow Configuration

Update workflow to use self-hosted runners:

```yaml
jobs:
  e2e-cross-platform:
    strategy:
      matrix:
        include:
          - os: linux
            runs-on: [self-hosted, linux, e2e]
          - os: macos
            runs-on: [self-hosted, macos, e2e]
          - os: windows
            runs-on: [self-hosted, windows, e2e]
    runs-on: ${{ matrix.runs-on }}
    steps:
      - uses: actions/checkout@v4
      # ... rest of steps
```

## Security Considerations

1. **Network Isolation**: Run on isolated network if possible
2. **Limited Access**: Use repository-level runners, not org-level
3. **Clean Environment**: Consider ephemeral runners or regular cleanup
4. **Secrets**: Never log or expose GitHub tokens
5. **Updates**: Keep runner and OS updated

## Troubleshooting

### Runner Not Connecting
```bash
# Check runner status
./svc.sh status

# View logs
cat _diag/Runner_*.log
```

### Audio Device Not Found
```bash
# Linux: Check PipeWire
pw-cli list-objects Node | grep -i audio

# macOS: Check audio devices
system_profiler SPAudioDataType

# Windows: Check sound devices
Get-WmiObject Win32_SoundDevice
```

### Permission Issues
```bash
# Linux: Add user to audio group
sudo usermod -aG audio $USER

# Re-login required
```

## Estimated Costs

For VPS-based runners:

| Provider | Spec | Monthly Cost |
|----------|------|--------------|
| Linux VM | 4 vCPU, 8GB RAM | ~$40 |
| macOS VM | 4 vCPU, 8GB RAM | ~$100 |
| Windows VM | 4 vCPU, 8GB RAM | ~$60 |

**Total for 3 runners: ~$200/month**

For 8-node mesh testing (additional):
- 5 additional Linux VMs: ~$200/month
- **Total with mesh: ~$400/month**

## References

- [GitHub Actions Self-Hosted Runners](https://docs.github.com/en/actions/hosting-your-own-runners)
- [PipeWire Documentation](https://pipewire.org/)
- [BlackHole Audio](https://existential.audio/blackhole/)
- [VB-Audio Virtual Cable](https://vb-audio.com/Cable/)
