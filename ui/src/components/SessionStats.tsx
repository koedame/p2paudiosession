/**
 * SessionStats - Detailed session statistics display
 *
 * Shows network stats and latency breakdown similar to CLI output.
 */

import type { NetworkStats, DetailedLatency } from "../lib/tauri";
import "./SessionStats.css";

export interface SessionStatsProps {
  network: NetworkStats | null;
  latency: DetailedLatency | null;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatDuration(seconds: number): string {
  if (seconds < 60) return `${seconds} sec`;
  const mins = Math.floor(seconds / 60);
  const secs = seconds % 60;
  return `${mins}:${secs.toString().padStart(2, "0")}`;
}

export function SessionStats({ network, latency }: SessionStatsProps) {
  if (!network || !latency) {
    return (
      <div className="session-stats session-stats--empty">
        <p>Waiting for connection...</p>
      </div>
    );
  }

  return (
    <div className="session-stats">
      {/* Network Section */}
      <section className="session-stats__section">
        <h3 className="session-stats__section-title">Network</h3>
        <div className="session-stats__grid">
          <div className="session-stats__item">
            <span className="session-stats__label">RTT</span>
            <span className="session-stats__value">{network.rtt_ms.toFixed(2)} ms</span>
          </div>
          <div className="session-stats__item">
            <span className="session-stats__label">Jitter</span>
            <span className="session-stats__value">{network.jitter_ms.toFixed(2)} ms</span>
          </div>
          <div className="session-stats__item">
            <span className="session-stats__label">Packet Loss</span>
            <span className="session-stats__value">{network.packet_loss_percent.toFixed(1)} %</span>
          </div>
          <div className="session-stats__item">
            <span className="session-stats__label">Uptime</span>
            <span className="session-stats__value">{formatDuration(Number(network.uptime_seconds))}</span>
          </div>
        </div>
      </section>

      {/* Latency Breakdown Section */}
      <section className="session-stats__section">
        <h3 className="session-stats__section-title">Latency Breakdown</h3>

        {/* Upstream */}
        <div className="session-stats__latency-group">
          <h4 className="session-stats__latency-title">
            Upstream (You → Peer)
          </h4>
          <div className="session-stats__latency-items">
            {latency.upstream.map((component, index) => (
              <div key={index} className="session-stats__latency-item">
                <span className="session-stats__latency-name">{component.name}</span>
                <span className="session-stats__latency-value">
                  {component.ms.toFixed(2)} ms
                  {component.info && (
                    <span className="session-stats__latency-info">({component.info})</span>
                  )}
                </span>
              </div>
            ))}
            <div className="session-stats__latency-item session-stats__latency-item--total">
              <span className="session-stats__latency-name">Total</span>
              <span className="session-stats__latency-value">{latency.upstream_total_ms.toFixed(2)} ms</span>
            </div>
          </div>
        </div>

        {/* Downstream */}
        <div className="session-stats__latency-group">
          <h4 className="session-stats__latency-title">
            Downstream (Peer → You)
          </h4>
          <div className="session-stats__latency-items">
            {latency.downstream.map((component, index) => (
              <div key={index} className="session-stats__latency-item">
                <span className="session-stats__latency-name">{component.name}</span>
                <span className="session-stats__latency-value">
                  {component.ms.toFixed(2)} ms
                  {component.info && (
                    <span className="session-stats__latency-info">({component.info})</span>
                  )}
                </span>
              </div>
            ))}
            <div className="session-stats__latency-item session-stats__latency-item--total">
              <span className="session-stats__latency-name">Total</span>
              <span className="session-stats__latency-value">{latency.downstream_total_ms.toFixed(2)} ms</span>
            </div>
          </div>
        </div>
      </section>

      {/* Summary */}
      <section className="session-stats__section">
        <h3 className="session-stats__section-title">Summary</h3>
        <div className="session-stats__summary">
          <div className="session-stats__summary-item">
            <span className="session-stats__label">Upstream</span>
            <span className="session-stats__value session-stats__value--highlight">
              {latency.upstream_total_ms.toFixed(2)} ms
            </span>
          </div>
          <div className="session-stats__summary-item">
            <span className="session-stats__label">Downstream</span>
            <span className="session-stats__value session-stats__value--highlight">
              {latency.downstream_total_ms.toFixed(2)} ms
            </span>
          </div>
          <div className="session-stats__summary-item">
            <span className="session-stats__label">Round-trip</span>
            <span className="session-stats__value session-stats__value--highlight">
              {latency.roundtrip_total_ms.toFixed(2)} ms
            </span>
          </div>
        </div>
      </section>

      {/* Packets */}
      <section className="session-stats__section">
        <h3 className="session-stats__section-title">Packets</h3>
        <div className="session-stats__grid">
          <div className="session-stats__item">
            <span className="session-stats__label">Sent</span>
            <span className="session-stats__value">{network.packets_sent.toLocaleString()}</span>
          </div>
          <div className="session-stats__item">
            <span className="session-stats__label">Received</span>
            <span className="session-stats__value">{network.packets_received.toLocaleString()}</span>
          </div>
          <div className="session-stats__item">
            <span className="session-stats__label">Bytes sent</span>
            <span className="session-stats__value">{formatBytes(Number(network.bytes_sent))}</span>
          </div>
          <div className="session-stats__item">
            <span className="session-stats__label">Bytes received</span>
            <span className="session-stats__value">{formatBytes(Number(network.bytes_received))}</span>
          </div>
        </div>
      </section>
    </div>
  );
}

export default SessionStats;
