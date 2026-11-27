/**
 * Pure translation from the engine's status view to something the UI can
 * render.
 *
 * Kept free of React and of the Tauri import so it can be unit-tested directly.
 * The UI must never infer engine state: every value here comes from a field the
 * engine actually sent, or is an explicit "we could not read it" state.
 */

export type ConnectionKind =
  | 'loading'
  | 'connected'
  | 'unavailable'
  | 'protocolMismatch'
  | 'permissionDenied'
  | 'engineError'
  | 'connectionFailed';

/** The status object the engine returns inside a `connected` view. */
export interface EngineStatusPayload {
  state: string;
  engineVersion: string | null;
  protocolVersion: number;
  uptimeMs: number;
  surfaceCount: number;
  isRunning: boolean;
  paused: boolean;
}

/** What the UI renders. */
export interface ViewModel {
  kind: ConnectionKind;
  /** Short label, e.g. "Engine connected". */
  label: string;
  /** One sentence explaining the state in plain language. */
  explanation: string;
  /** A non-colour cue shown beside the label (never colour alone). */
  glyph: string;
  /** Whether the engine answered. */
  reachable: boolean;
  /** Present only for `connected`. */
  status?: EngineStatusPayload;
  /** Present when there is a diagnostic string worth showing. */
  detail?: string;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function readString(source: Record<string, unknown>, key: string): string | null {
  const value = source[key];
  return typeof value === 'string' ? value : null;
}

function readNumber(source: Record<string, unknown>, key: string): number | null {
  const value = source[key];
  return typeof value === 'number' && Number.isFinite(value) ? value : null;
}

function readBoolean(source: Record<string, unknown>, key: string): boolean | null {
  const value = source[key];
  return typeof value === 'boolean' ? value : null;
}

/** The initial view, before the first round-trip completes. */
export function initialView(): ViewModel {
  return {
    kind: 'loading',
    label: 'Checking the engine',
    explanation: 'Looking for the DiscoDesk engine on this machine.',
    glyph: '\u2026',
    reachable: false,
  };
}

/** A view for a response we could not understand. */
export function malformedView(why: string): ViewModel {
  return {
    kind: 'connectionFailed',
    label: 'Unexpected response',
    explanation:
      'The engine answered, but the Studio could not understand the reply. The versions may not match.',
    glyph: '!',
    reachable: false,
    detail: why,
  };
}

function readStatus(source: Record<string, unknown>): EngineStatusPayload | null {
  const state = readString(source, 'state');
  const protocolVersion = readNumber(source, 'protocolVersion');
  const uptimeMs = readNumber(source, 'uptimeMs');
  const surfaceCount = readNumber(source, 'surfaceCount');
  const isRunning = readBoolean(source, 'isRunning');
  const paused = readBoolean(source, 'paused');

  if (
    state === null ||
    protocolVersion === null ||
    uptimeMs === null ||
    surfaceCount === null ||
    isRunning === null ||
    paused === null
  ) {
    return null;
  }

  return {
    state,
    engineVersion: readString(source, 'engineVersion'),
    protocolVersion,
    uptimeMs,
    surfaceCount,
    isRunning,
    paused,
  };
}

/**
 * Translates an `EngineView` from the Rust side into a view model.
 *
 * Every branch is reachable from a real condition the engine or the transport
 * can produce; unknown input is reported as "unexpected response" rather than
 * being guessed at.
 */
export function describeEngineView(raw: unknown): ViewModel {
  if (!isRecord(raw)) {
    return malformedView('the response was not an object');
  }

  const kind = readString(raw, 'kind');
  if (kind === null) {
    return malformedView('the response had no kind tag');
  }

  switch (kind) {
    case 'connected': {
      const status = isRecord(raw.status) ? readStatus(raw.status) : null;
      if (status === null) {
        return malformedView('the status object was missing required fields');
      }
      const verb = status.paused ? 'paused' : status.isRunning ? 'running' : 'idle';
      return {
        kind: 'connected',
        label: 'Engine connected',
        explanation: `The engine is ${verb} (${status.state}).`,
        glyph: '\u2713',
        reachable: true,
        status,
      };
    }

    case 'unavailable':
      return {
        kind: 'unavailable',
        label: 'Engine not running',
        explanation:
          'Nothing is listening for the Studio. Start the DiscoDesk engine, then leave this window open; it reconnects automatically.',
        glyph: '\u25CB',
        reachable: false,
        detail: readString(raw, 'detail') ?? undefined,
      };

    case 'protocolMismatch': {
      const expected = readNumber(raw, 'expected');
      const found = readNumber(raw, 'found');
      const detail =
        expected !== null && found !== null
          ? `The engine speaks protocol ${found}; this Studio speaks ${expected}.`
          : undefined;
      return {
        kind: 'protocolMismatch',
        label: 'Version mismatch',
        explanation:
          'The engine and the Studio were built from different versions. Update the one that is older so they use the same protocol.',
        glyph: '\u21C4',
        reachable: false,
        detail,
      };
    }

    case 'permissionDenied':
      return {
        kind: 'permissionDenied',
        label: 'Not permitted',
        explanation:
          'The engine exists but this user account is not allowed to connect to it.',
        glyph: '\u26BF',
        reachable: false,
        detail: readString(raw, 'detail') ?? undefined,
      };

    case 'engineError': {
      const code = readNumber(raw, 'code');
      const message = readString(raw, 'message');
      return {
        kind: 'engineError',
        label: 'Engine reported a problem',
        explanation: 'The engine answered the status request with an error.',
        glyph: '!',
        reachable: true,
        detail: [code === null ? null : `code ${code}`, message]
          .filter((part): part is string => part !== null && part.length > 0)
          .join(': '),
      };
    }

    case 'connectionFailed':
      return {
        kind: 'connectionFailed',
        label: 'Connection failed',
        explanation:
          'The Studio reached something on the endpoint but the exchange did not complete.',
        glyph: '!',
        reachable: false,
        detail: readString(raw, 'detail') ?? undefined,
      };

    default:
      return malformedView(`unknown view kind '${kind}'`);
  }
}

/** Maps a rejected `invoke` call to a view. */
export function describeInvokeError(error: unknown): ViewModel {
  const detail = error instanceof Error ? error.message : String(error);
  return {
    kind: 'connectionFailed',
    label: 'Studio could not reach the engine bridge',
    explanation:
      'The Studio’s own request failed before the engine could answer. This usually means the engine status command failed.',
    glyph: '!',
    reachable: false,
    detail,
  };
}

/** Formats a millisecond duration as a short human string. */
export function formatUptime(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) {
    return 'unknown';
  }
  const totalSeconds = Math.floor(ms / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  }
  if (minutes > 0) {
    return `${minutes}m ${seconds}s`;
  }
  return `${seconds}s`;
}
