/**
 * ConnectionIndicator component
 *
 * Displays real-time connection status with optional latency value.
 */

import { useTranslation } from 'react-i18next';
import {
  DisconnectedIcon,
  ConnectingIcon,
  ConnectedIcon,
  UnstableIcon,
  ErrorIcon,
} from './icons';
import './ConnectionIndicator.css';

export type ConnectionStatus =
  | 'disconnected'
  | 'connecting'
  | 'connected'
  | 'unstable'
  | 'error';

export interface ConnectionIndicatorProps {
  /** Connection status */
  status: ConnectionStatus;

  /** RTT (round-trip time) in milliseconds (deprecated, use upstreamLatencyMs/downstreamLatencyMs) */
  latencyMs?: number;

  /** Upstream latency (self -> peer) in milliseconds */
  upstreamLatencyMs?: number;

  /** Downstream latency (peer -> self) in milliseconds */
  downstreamLatencyMs?: number;

  /** Whether to show latency value */
  showLatency?: boolean;

  /** Component size */
  size?: 'sm' | 'md' | 'lg';

  /** Click handler (e.g., navigate to connection details) */
  onClick?: () => void;
}

const iconSizes = {
  sm: 12,
  md: 16,
  lg: 20,
};

function getIcon(status: ConnectionStatus, size: number) {
  const iconProps = { size, className: 'connection-indicator__icon' };

  switch (status) {
    case 'disconnected':
      return <DisconnectedIcon {...iconProps} />;
    case 'connecting':
      return <ConnectingIcon {...iconProps} />;
    case 'connected':
      return <ConnectedIcon {...iconProps} />;
    case 'unstable':
      return <UnstableIcon {...iconProps} />;
    case 'error':
      return <ErrorIcon {...iconProps} />;
  }
}

export function ConnectionIndicator({
  status,
  latencyMs,
  upstreamLatencyMs,
  downstreamLatencyMs,
  showLatency = true,
  size = 'md',
  onClick,
}: ConnectionIndicatorProps) {
  const { t } = useTranslation();

  const statusText = t(`status.${status}`);

  // Build latency text: prefer upstream/downstream if available
  let latencyText: string | null = null;
  if (showLatency) {
    if (upstreamLatencyMs !== undefined && downstreamLatencyMs !== undefined) {
      // Show both directions: "↑12ms ↓12ms"
      latencyText = `↑${Math.round(upstreamLatencyMs)}ms ↓${Math.round(downstreamLatencyMs)}ms`;
    } else if (latencyMs !== undefined) {
      // Fallback to single RTT value
      latencyText = t('status.latency', { ms: Math.round(latencyMs) });
    }
  }

  // Build aria-label for screen readers
  const ariaLabel = latencyText
    ? `${statusText} (${latencyText})`
    : statusText;

  const classNames = [
    'connection-indicator',
    `connection-indicator--${status}`,
    `connection-indicator--${size}`,
    onClick ? 'connection-indicator--clickable' : '',
  ]
    .filter(Boolean)
    .join(' ');

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (onClick && (e.key === 'Enter' || e.key === ' ')) {
      e.preventDefault();
      onClick();
    }
  };

  return (
    <div
      role="status"
      aria-live="polite"
      aria-label={ariaLabel}
      className={classNames}
      onClick={onClick}
      onKeyDown={handleKeyDown}
      tabIndex={onClick ? 0 : undefined}
    >
      <span aria-hidden="true">{getIcon(status, iconSizes[size])}</span>
      <span className="connection-indicator__text">{statusText}</span>
      {latencyText && (
        <span className="connection-indicator__latency">({latencyText})</span>
      )}
    </div>
  );
}

export default ConnectionIndicator;
