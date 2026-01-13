---
sidebar_label: ADR-010 E2E Test Infrastructure
sidebar_position: 10
---

# ADR-010: E2E Test Infrastructure

## Status

Accepted

## Context

jamjam is a P2P audio communication application where audio quality and low latency are critical requirements (as defined in ADR-008). Manual testing is insufficient to guarantee:

1. Audio quality meets PESQ/MOS thresholds across all presets
2. Cross-platform connections work reliably (Linux ↔ macOS ↔ Windows)
3. Multi-user scenarios (up to 8 participants) remain stable
4. Latency stays within ADR-008 specifications

We need an automated E2E test infrastructure that:
- Runs on every PR (lightweight tests)
- Runs nightly (full test suite)
- Provides objective audio quality metrics
- Tests cross-platform interoperability
- Scales to 8-node mesh topology

## Decision

### Test Architecture

Four-layer test pyramid:

```
┌───────────────────────────────────────┐
│   System E2E (Nightly, VPS cluster)   │  PESQ, 8-node mesh
├───────────────────────────────────────┤
│   Integration E2E (PR, self-hosted)   │  Cross-platform
├───────────────────────────────────────┤
│   Component Integration (PR, GH)      │  Audio+Network loopback
├───────────────────────────────────────┤
│   Unit Tests (Every commit)           │  117+ tests
└───────────────────────────────────────┘
```

### Infrastructure

| Environment | Runner Type | Tests |
|------------|-------------|-------|
| GitHub Actions | ubuntu-latest | Unit, loopback, network-local |
| Self-hosted | Linux/macOS/Windows | Cross-platform integration |
| VPS Cluster | 8+ nodes | Full mesh, PESQ evaluation |

### Virtual Audio Devices

| Platform | Solution |
|----------|----------|
| Linux | PipeWire null-audio-sink |
| macOS | BlackHole 2ch |
| Windows | VB-Audio Virtual Cable |

### Audio Quality Metrics

- **PESQ (ITU-T P.862)**: MOS-LQO score (1.0 - 4.5)
- **Latency**: Cross-correlation measurement
- **Packet loss**: Counted during transmission

Thresholds from ADR-008:

| Preset | Min MOS | Max Latency |
|--------|---------|-------------|
| zero-latency | 4.0 | 2ms |
| ultra-low-latency | 3.8 | 5ms |
| balanced | 3.5 | 15ms |
| high-quality | 4.2 | 30ms |

### Feature Flags

```toml
[features]
e2e-loopback = []        # Audio loopback tests (no network)
e2e-network-local = []   # Local network tests (localhost)
e2e-remote = []          # Remote node tests (VPS cluster)
e2e-full = ["e2e-loopback", "e2e-network-local", "e2e-remote"]
```

### Directory Structure

```
tests/e2e/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── orchestrator.rs      # Multi-node coordination
│   ├── node.rs              # Remote node management
│   ├── audio_injection.rs   # Virtual audio control
│   ├── quality.rs           # PESQ/latency evaluation
│   └── scenarios/
│       ├── loopback.rs
│       ├── two_node.rs
│       ├── cross_platform.rs
│       └── eight_node.rs
└── scripts/
    ├── setup-virtual-audio-linux.sh
    ├── setup-virtual-audio-macos.sh
    └── setup-virtual-audio-windows.ps1
```

### Workflow Triggers

| Workflow | Trigger | Tests |
|----------|---------|-------|
| e2e-pr.yml | PR, push to main | loopback, network-local |
| e2e-nightly.yml | Daily 2:00 AM JST | All presets, two-node |
| e2e-nightly.yml (manual) | workflow_dispatch | Full matrix, 8-node |

## Consequences

### Benefits

1. **Objective quality assurance**: PESQ provides industry-standard audio quality metrics
2. **Cross-platform confidence**: Automated testing of all OS combinations
3. **Regression detection**: Quality degradation caught before merge
4. **Scalability testing**: 8-node mesh validates production scenarios

### Costs

1. **Infrastructure cost**: ~$300/month for VPS cluster (nightly tests)
2. **Maintenance**: Virtual audio setup varies by OS
3. **Complexity**: Multi-node orchestration requires careful synchronization

### Risks

1. **Flaky tests**: Network tests may be sensitive to timing
   - Mitigation: Use retry logic, increase timeouts
2. **Platform differences**: Audio behavior varies by OS
   - Mitigation: Platform-specific thresholds if needed
3. **CI environment limitations**: GitHub Actions lacks real audio devices
   - Mitigation: Use virtual audio devices for PR tests

## Implementation Notes

### Phase 1 (Complete)
- [x] Create tests/e2e/ directory structure
- [x] Implement virtual audio setup scripts
- [x] Add feature flags to Cargo.toml
- [x] Create loopback test scenarios
- [x] Create e2e-pr.yml workflow

### Phase 2 (Complete)
- [x] Integrate PESQ evaluation (Python wrapper + correlation-based fallback)
- [x] Audio injection/capture with virtual devices (VirtualAudioManager)
- [x] Two-node localhost tests with real audio path
- [x] Reference audio fixtures generator (sine, sweep, noise, impulse, speech-like)
- [x] Latency measurement via cross-correlation
- [x] Self-hosted runner setup documentation
- [x] e2e-nightly.yml workflow for full test suite

### Phase 3 (Future)
- [ ] Set up self-hosted runners (Linux/macOS/Windows)
- [ ] Cross-platform test orchestration
- [ ] 8-node VPS cluster provisioning

## References

- ADR-008: Zero-latency mode and audio quality requirements
- ITU-T P.862: PESQ algorithm specification
- [PipeWire](https://pipewire.org/): Linux audio framework
- [BlackHole](https://existential.audio/blackhole/): macOS virtual audio
- [VB-Audio](https://vb-audio.com/Cable/): Windows virtual audio
