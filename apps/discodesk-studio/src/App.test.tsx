import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen, waitFor } from '@testing-library/react';

vi.mock('./api', () => ({
  fetchEngineStatus: vi.fn(),
  fetchEndpoint: vi.fn(),
  disconnectFromEngine: vi.fn(),
}));

import App from './App';
import { disconnectFromEngine, fetchEndpoint, fetchEngineStatus } from './api';

const mockedStatus = vi.mocked(fetchEngineStatus);
const mockedEndpoint = vi.mocked(fetchEndpoint);
const mockedDisconnect = vi.mocked(disconnectFromEngine);

const connectedPayload = {
  kind: 'connected',
  status: {
    state: 'IdleReady',
    engineVersion: '0.1.0',
    protocolVersion: 1,
    uptimeMs: 65000,
    surfaceCount: 0,
    isRunning: true,
    paused: false,
  },
};

beforeEach(() => {
  mockedStatus.mockReset();
  mockedEndpoint.mockReset();
  mockedDisconnect.mockReset();
  mockedEndpoint.mockResolvedValue(String.raw`\\.\pipe\discodesk-engine-test`);
  mockedDisconnect.mockResolvedValue(undefined);
});

afterEach(() => {
  cleanup();
});

describe('App', () => {
  it('renders the loading state before any status arrives', () => {
    // A promise that never settles keeps the component in its initial state.
    mockedStatus.mockReturnValue(new Promise(() => undefined));

    render(<App />);

    expect(screen.getByText('Checking the engine')).toBeInTheDocument();
    expect(screen.getByText('Engine state')).toBeInTheDocument();
    expect(screen.getByText('unavailable while disconnected')).toBeInTheDocument();
  });

  it('renders the connected state and the engine details', async () => {
    mockedStatus.mockResolvedValue(connectedPayload);

    render(<App />);

    await waitFor(() => {
      expect(screen.getByText('Engine connected')).toBeInTheDocument();
    });
    expect(screen.getByText('IdleReady')).toBeInTheDocument();
    expect(screen.getByText('0.1.0')).toBeInTheDocument();
    expect(screen.getByText('1m 5s')).toBeInTheDocument();
    // The status region advertises the machine-readable kind for styling and
    // tests, never as the only carrier of meaning.
    const region = screen.getByText('Engine connected').closest('.status');
    expect(region?.getAttribute('data-kind')).toBe('connected');
  });

  it('renders the disconnected state when the engine is absent', async () => {
    mockedStatus.mockResolvedValue({ kind: 'unavailable', detail: 'pipe not found' });

    render(<App />);

    await waitFor(() => {
      expect(screen.getByText('Engine not running')).toBeInTheDocument();
    });
    expect(screen.getByText('pipe not found')).toBeInTheDocument();
    const region = screen.getByText('Engine not running').closest('.status');
    expect(region?.getAttribute('data-kind')).toBe('unavailable');
    expect(region?.getAttribute('data-reachable')).toBe('false');
  });

  it('renders a version mismatch with both protocol numbers', async () => {
    mockedStatus.mockResolvedValue({ kind: 'protocolMismatch', expected: 1, found: 9 });

    render(<App />);

    await waitFor(() => {
      expect(screen.getByText('Version mismatch')).toBeInTheDocument();
    });
    expect(screen.getByText(/speaks protocol 9/)).toBeInTheDocument();
  });

  it('renders a bridge failure when invoke rejects', async () => {
    mockedStatus.mockRejectedValue(new Error('command unavailable'));

    render(<App />);

    await waitFor(() => {
      expect(screen.getByText(/could not reach the engine bridge/i)).toBeInTheDocument();
    });
    expect(screen.getByText('command unavailable')).toBeInTheDocument();
  });

  it('releases the connection when the Studio unmounts', async () => {
    mockedStatus.mockResolvedValue(connectedPayload);

    const { unmount } = render(<App />);
    await waitFor(() => {
      expect(screen.getByText('Engine connected')).toBeInTheDocument();
    });

    unmount();

    expect(mockedDisconnect).toHaveBeenCalledTimes(1);
  });

  it('never claims the engine is connected when the status call fails', async () => {
    mockedStatus.mockRejectedValue(new Error('boom'));

    render(<App />);

    await waitFor(() => {
      expect(screen.getByText(/could not reach the engine bridge/i)).toBeInTheDocument();
    });
    expect(screen.queryByText('Engine connected')).not.toBeInTheDocument();
  });
});
