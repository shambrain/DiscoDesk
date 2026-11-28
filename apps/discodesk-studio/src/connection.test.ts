import { describe, expect, it } from 'vitest';
import {
  describeAudio,
  describeEngineView,
  describeInvokeError,
  formatUptime,
  initialView,
  malformedView,
} from './connection';

const validStatus = {
  state: 'IdleReady',
  engineVersion: '0.1.0',
  protocolVersion: 1,
  uptimeMs: 65000,
  surfaceCount: 0,
  isRunning: true,
  paused: false,
};

describe('initialView', () => {
  it('is the loading state before the first round-trip', () => {
    const view = initialView();
    expect(view.kind).toBe('loading');
    expect(view.reachable).toBe(false);
    expect(view.status).toBeUndefined();
  });
});

describe('describeEngineView — connected', () => {
  it('reports the engine values it was given', () => {
    const view = describeEngineView({ kind: 'connected', status: validStatus });
    expect(view.kind).toBe('connected');
    expect(view.reachable).toBe(true);
    expect(view.status?.state).toBe('IdleReady');
    expect(view.status?.protocolVersion).toBe(1);
    expect(view.status?.surfaceCount).toBe(0);
    expect(view.label).toContain('connected');
  });

  it('says paused when the engine is paused', () => {
    const view = describeEngineView({
      kind: 'connected',
      status: { ...validStatus, isRunning: false, paused: true },
    });
    expect(view.explanation).toContain('paused');
  });

  it('rejects a status object with missing required fields', () => {
    const view = describeEngineView({
      kind: 'connected',
      status: { state: 'IdleReady' },
    });
    expect(view.kind).toBe('connectionFailed');
    expect(view.reachable).toBe(false);
  });

  it('rejects a non-numeric protocol version', () => {
    const view = describeEngineView({
      kind: 'connected',
      status: { ...validStatus, protocolVersion: 'one' },
    });
    expect(view.kind).toBe('connectionFailed');
  });
});

describe('describeEngineView — disconnected states', () => {
  it('maps unavailable to "engine not running" and keeps the detail', () => {
    const view = describeEngineView({
      kind: 'unavailable',
      detail: 'could not open the named pipe',
    });
    expect(view.kind).toBe('unavailable');
    expect(view.reachable).toBe(false);
    expect(view.label).toMatch(/not running/i);
    expect(view.detail).toContain('named pipe');
    expect(view.status).toBeUndefined();
  });

  it('maps a protocol mismatch and states both versions', () => {
    const view = describeEngineView({ kind: 'protocolMismatch', expected: 1, found: 4 });
    expect(view.kind).toBe('protocolMismatch');
    expect(view.reachable).toBe(false);
    expect(view.detail).toContain('4');
    expect(view.detail).toContain('1');
  });

  it('maps permission denied without claiming the engine is absent', () => {
    const view = describeEngineView({ kind: 'permissionDenied', detail: 'access denied' });
    expect(view.kind).toBe('permissionDenied');
    expect(view.explanation).not.toMatch(/not running/i);
  });

  it('maps an engine error and includes the code', () => {
    const view = describeEngineView({ kind: 'engineError', code: 1015, message: 'later gate' });
    expect(view.kind).toBe('engineError');
    expect(view.reachable).toBe(true);
    expect(view.detail).toContain('1015');
    expect(view.detail).toContain('later gate');
  });

  it('maps a connection failure', () => {
    const view = describeEngineView({ kind: 'connectionFailed', detail: 'io error' });
    expect(view.kind).toBe('connectionFailed');
    expect(view.detail).toBe('io error');
  });
});

describe('describeEngineView — hostile or unexpected input', () => {
  it('rejects null', () => {
    expect(describeEngineView(null).kind).toBe('connectionFailed');
  });

  it('rejects an array', () => {
    expect(describeEngineView([]).kind).toBe('connectionFailed');
  });

  it('rejects a missing kind tag', () => {
    const view = describeEngineView({ status: validStatus });
    expect(view.kind).toBe('connectionFailed');
    expect(view.detail).toContain('kind');
  });

  it('rejects an unknown kind rather than guessing', () => {
    const view = describeEngineView({ kind: 'somethingNew' });
    expect(view.kind).toBe('connectionFailed');
    expect(view.detail).toContain('somethingNew');
  });
});

describe('describeInvokeError', () => {
  it('surfaces the message of an Error', () => {
    const view = describeInvokeError(new Error('command failed'));
    expect(view.kind).toBe('connectionFailed');
    expect(view.detail).toContain('command failed');
  });

  it('surfaces a plain string rejection', () => {
    const view = describeInvokeError('denied');
    expect(view.detail).toContain('denied');
  });
});

describe('malformedView', () => {
  it('is a connection failure that never claims reachability', () => {
    const view = malformedView('bad shape');
    expect(view.kind).toBe('connectionFailed');
    expect(view.reachable).toBe(false);
  });
});

describe('formatUptime', () => {
  it('formats seconds', () => {
    expect(formatUptime(5000)).toBe('5s');
  });

  it('formats minutes', () => {
    expect(formatUptime(65000)).toBe('1m 5s');
  });

  it('formats hours', () => {
    expect(formatUptime(3 * 3600 * 1000 + 60 * 1000)).toBe('3h 1m');
  });

  it('rejects negative and non-finite values', () => {
    expect(formatUptime(-1)).toBe('unknown');
    expect(formatUptime(Number.POSITIVE_INFINITY)).toBe('unknown');
  });
});

describe('describeAudio', () => {
  const streaming = {
    engineState: 'AudioActive',
    audio: {
      state: 'streaming',
      backend: 'wasapi-loopback',
      deviceName: 'OUT 1-2 (test)',
      sampleRate: 48000,
      channels: 2,
      encoding: 'f32',
      stats: { packets: 10, droppedSamples: 2, restarts: 1, lastError: null },
      dspLastMicros: 25,
      dspMaxMicros: 80,
      dspFrames: 50,
      hopRateHz: 46.875,
      frame: {
        loudnessDbfs: -12,
        levelRms: 0.25,
        levelPeak: 0.5,
        spectralFlux: 0.3,
        onset: false,
        bands: [0.2, 0.9, 0.4, 0.6, 0.1, 0.3],
        bandEnvelopes: [0.15, 0.8, 0.35, 0.5, 0.05, 0.25],
      },
    },
  };

  it('maps a streaming response into six bands and metrics', () => {
    const view = describeAudio(streaming);
    expect(view.kind).toBe('running');
    expect(view.label).toBe('Listening');
    expect(view.device).toBe('OUT 1-2 (test)');
    expect(view.format).toContain('48000 Hz');
    expect(view.bands).toHaveLength(6);
    expect(view.bands[1]?.envelope).toBeCloseTo(0.8);
    expect(view.bands[1]?.raw).toBeCloseTo(0.9);
    expect(view.levelDbfs).toBeCloseTo(-12);
    expect(view.dropped).toBe(2);
    expect(view.restarts).toBe(1);
    expect(view.dspMaxMicros).toBe(80);
  });

  it('clamps out-of-range band values instead of trusting them', () => {
    const wild = structuredClone(streaming);
    wild.audio.frame.bands = [5, -1, 0.5, 0.5, 0.5, 0.5] as never;
    wild.audio.frame.bandEnvelopes = [5, -1, 0.5, 0.5, 0.5, 0.5] as never;
    const view = describeAudio(wild);
    expect(view.bands[0]?.envelope).toBe(1);
    expect(view.bands[1]?.envelope).toBe(0);
  });

  it('reports silence distinctly and explains the missing frame', () => {
    const silent = structuredClone(streaming);
    silent.audio.state = 'silent';
    silent.audio.frame = null as never;
    const view = describeAudio(silent);
    expect(view.kind).toBe('silent');
    expect(view.detail).toContain('nothing is playing');
    expect(view.bands.every((band) => band.envelope === 0)).toBe(true);
  });

  it('surfaces the engine error text for an unavailable capture', () => {
    const broken = {
      engineState: 'Listening',
      audio: {
        state: 'unavailable',
        stats: { lastError: 'no default output device is available' },
      },
    };
    const view = describeAudio(broken);
    expect(view.kind).toBe('unavailable');
    expect(view.detail).toContain('no default output device');
  });

  it('rejects a response with no audio object', () => {
    expect(describeAudio({}).kind).toBe('error');
    expect(describeAudio(null).kind).toBe('error');
    expect(describeAudio('nonsense').kind).toBe('error');
  });
});