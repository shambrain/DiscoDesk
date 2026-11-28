import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen, waitFor } from '@testing-library/react';

vi.mock('./api', () => ({
  fetchEngineStatus: vi.fn(),
  fetchAudioTelemetry: vi.fn(),
  fetchEndpoint: vi.fn(),
  disconnectFromEngine: vi.fn(),
}));

import App from './App';
import {
  disconnectFromEngine,
  fetchAudioTelemetry,
  fetchEndpoint,
  fetchEngineStatus,
} from './api';

const mockedStatus = vi.mocked(fetchEngineStatus);
const mockedAudio = vi.mocked(fetchAudioTelemetry);
const mockedEndpoint = vi.mocked(fetchEndpoint);
const mockedDisconnect = vi.mocked(disconnectFromEngine);

const audioPayload = {
  engineState: 'AudioActive',
  audio: {
    state: 'streaming',
    backend: 'wasapi-loopback',
    deviceName: 'OUT 1-2 (test device)',
    sampleRate: 44100,
    channels: 2,
    encoding: 'f32',
    bands: undefined,
    stats: { packets: 42, droppedSamples: 0, restarts: 0, lastError: null },
    dspLastMicros: 30,
    dspMaxMicros: 60,
    dspFrames: 100,
    hopRateHz: 43.07,
    frame: {
      loudnessDbfs: -14.5,
      levelRms: 0.19,
      levelPeak: 0.49,
      spectralFlux: 0.2,
      onset: true,
      bands: [0.1, 0.8, 0.6, 0.7, 0.2, 0.5],
      bandEnvelopes: [0.1, 0.75, 0.55, 0.65, 0.18, 0.45],
    },
  },
};

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
  mockedAudio.mockReset();
  mockedEndpoint.mockReset();
  mockedDisconnect.mockReset();
  mockedAudio.mockResolvedValue(audioPayload);
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

  it('renders live audio telemetry with all six bands', async () => {
    mockedStatus.mockResolvedValue(connectedPayload);

    render(<App />);

    await waitFor(() => {
      expect(screen.getByText('Audio')).toBeInTheDocument();
    });
    expect(screen.getByText('OUT 1-2 (test device)')).toBeInTheDocument();
    expect(screen.getByText(/44100 Hz/)).toBeInTheDocument();
    for (const label of [
      'Sub-bass',
      'Punch',
      'Warmth',
      'Presence',
      'Brilliance',
      'Air',
    ]) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
    // The value is exposed as text, not only as a bar width.
    expect(screen.getByText('0.75')).toBeInTheDocument();
    expect(screen.getByText(/-14\.5 dBFS/)).toBeInTheDocument();
    expect(screen.getByText(/onset/)).toBeInTheDocument();
  });

  it('renders audio as unavailable when the telemetry request fails', async () => {
    mockedStatus.mockResolvedValue(connectedPayload);
    mockedAudio.mockRejectedValue(new Error('no audio'));

    render(<App />);

    await waitFor(() => {
      expect(screen.getByText('Audio unavailable')).toBeInTheDocument();
    });
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
