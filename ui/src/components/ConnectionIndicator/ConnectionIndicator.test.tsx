/**
 * ConnectionIndicator component tests
 *
 * Test cases based on docs-spec/ui/components/connection-indicator.md
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import '@testing-library/jest-dom/vitest';
import { ConnectionIndicator, type ConnectionStatus } from './ConnectionIndicator';

// Mock i18next
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, options?: { ms?: number }) => {
      const translations: Record<string, string> = {
        'status.disconnected': 'Disconnected',
        'status.connecting': 'Connecting...',
        'status.connected': 'Connected',
        'status.unstable': 'Unstable',
        'status.error': 'Disconnected',
      };
      if (key === 'status.latency' && options?.ms !== undefined) {
        return `${options.ms}ms`;
      }
      return translations[key] || key;
    },
  }),
}));

describe('ConnectionIndicator', () => {
  const statuses: ConnectionStatus[] = [
    'disconnected',
    'connecting',
    'connected',
    'unstable',
    'error',
  ];

  describe('Status Display', () => {
    it.each(statuses)('displays correct text for %s status', (status: ConnectionStatus) => {
      render(<ConnectionIndicator status={status} />);
      const element = screen.getByRole('status');
      expect(element).toBeInTheDocument();
    });

    it('displays disconnected status correctly', () => {
      render(<ConnectionIndicator status="disconnected" />);
      expect(screen.getByText('Disconnected')).toBeInTheDocument();
    });

    it('displays connecting status correctly', () => {
      render(<ConnectionIndicator status="connecting" />);
      expect(screen.getByText('Connecting...')).toBeInTheDocument();
    });

    it('displays connected status correctly', () => {
      render(<ConnectionIndicator status="connected" />);
      expect(screen.getByText('Connected')).toBeInTheDocument();
    });

    it('displays unstable status correctly', () => {
      render(<ConnectionIndicator status="unstable" />);
      expect(screen.getByText('Unstable')).toBeInTheDocument();
    });

    it('displays error status correctly', () => {
      render(<ConnectionIndicator status="error" />);
      // Error status shows "Disconnected" text
      expect(screen.getByText('Disconnected')).toBeInTheDocument();
    });
  });

  describe('Latency Display', () => {
    it('displays latency when latencyMs is provided', () => {
      render(<ConnectionIndicator status="connected" latencyMs={15} />);
      expect(screen.getByText('(15ms)')).toBeInTheDocument();
    });

    it('does not display latency when latencyMs is undefined', () => {
      render(<ConnectionIndicator status="connected" />);
      expect(screen.queryByText(/ms\)/)).not.toBeInTheDocument();
    });

    it('hides latency when showLatency is false', () => {
      render(
        <ConnectionIndicator status="connected" latencyMs={15} showLatency={false} />
      );
      expect(screen.queryByText('(15ms)')).not.toBeInTheDocument();
    });

    it('shows latency when showLatency is true (default)', () => {
      render(<ConnectionIndicator status="connected" latencyMs={15} showLatency={true} />);
      expect(screen.getByText('(15ms)')).toBeInTheDocument();
    });
  });

  describe('Size Variants', () => {
    it('applies sm size class', () => {
      render(<ConnectionIndicator status="connected" size="sm" />);
      const element = screen.getByRole('status');
      expect(element.className).toContain('connection-indicator--sm');
    });

    it('applies md size class (default)', () => {
      render(<ConnectionIndicator status="connected" />);
      const element = screen.getByRole('status');
      expect(element.className).toContain('connection-indicator--md');
    });

    it('applies lg size class', () => {
      render(<ConnectionIndicator status="connected" size="lg" />);
      const element = screen.getByRole('status');
      expect(element.className).toContain('connection-indicator--lg');
    });
  });

  describe('Click Handler', () => {
    it('calls onClick when clicked', () => {
      const handleClick = vi.fn();
      render(<ConnectionIndicator status="connected" onClick={handleClick} />);

      const element = screen.getByRole('status');
      fireEvent.click(element);

      expect(handleClick).toHaveBeenCalledTimes(1);
    });

    it('does not have tabIndex when onClick is not provided', () => {
      render(<ConnectionIndicator status="connected" />);
      const element = screen.getByRole('status');
      expect(element).not.toHaveAttribute('tabIndex');
    });

    it('has tabIndex 0 when onClick is provided', () => {
      render(<ConnectionIndicator status="connected" onClick={() => {}} />);
      const element = screen.getByRole('status');
      expect(element).toHaveAttribute('tabIndex', '0');
    });

    it('triggers onClick on Enter key', () => {
      const handleClick = vi.fn();
      render(<ConnectionIndicator status="connected" onClick={handleClick} />);

      const element = screen.getByRole('status');
      fireEvent.keyDown(element, { key: 'Enter' });

      expect(handleClick).toHaveBeenCalledTimes(1);
    });

    it('triggers onClick on Space key', () => {
      const handleClick = vi.fn();
      render(<ConnectionIndicator status="connected" onClick={handleClick} />);

      const element = screen.getByRole('status');
      fireEvent.keyDown(element, { key: ' ' });

      expect(handleClick).toHaveBeenCalledTimes(1);
    });

    it('applies clickable class when onClick is provided', () => {
      render(<ConnectionIndicator status="connected" onClick={() => {}} />);
      const element = screen.getByRole('status');
      expect(element.className).toContain('connection-indicator--clickable');
    });
  });

  describe('Accessibility', () => {
    it('has role="status"', () => {
      render(<ConnectionIndicator status="connected" />);
      expect(screen.getByRole('status')).toBeInTheDocument();
    });

    it('has aria-live="polite"', () => {
      render(<ConnectionIndicator status="connected" />);
      const element = screen.getByRole('status');
      expect(element).toHaveAttribute('aria-live', 'polite');
    });

    it('has aria-label with status text', () => {
      render(<ConnectionIndicator status="connected" />);
      const element = screen.getByRole('status');
      expect(element).toHaveAttribute('aria-label', 'Connected');
    });

    it('has aria-label with status and latency', () => {
      render(<ConnectionIndicator status="connected" latencyMs={15} />);
      const element = screen.getByRole('status');
      expect(element).toHaveAttribute('aria-label', 'Connected (15ms)');
    });

    it('icon is hidden from screen readers', () => {
      render(<ConnectionIndicator status="connected" />);
      const element = screen.getByRole('status');
      const icon = element.querySelector('[aria-hidden="true"]');
      expect(icon).toBeInTheDocument();
    });
  });

  describe('CSS Classes', () => {
    it.each(statuses)('applies correct status class for %s', (status: ConnectionStatus) => {
      render(<ConnectionIndicator status={status} />);
      const element = screen.getByRole('status');
      expect(element.className).toContain(`connection-indicator--${status}`);
    });

    it('has base class', () => {
      render(<ConnectionIndicator status="connected" />);
      const element = screen.getByRole('status');
      expect(element.className).toContain('connection-indicator');
    });
  });
});
