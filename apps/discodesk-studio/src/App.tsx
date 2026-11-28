import { useCallback, useEffect, useRef, useState } from 'react';
import {
  disconnectFromEngine,
  fetchAudioTelemetry,
  fetchEndpoint,
  fetchEngineStatus,
} from './api';
import {
  describeAudio,
  describeEngineView,
  describeInvokeError,
  formatUptime,
  initialAudioView,
  initialView,
  type AudioViewModel,
  type ViewModel,
} from './connection';

/** How often the Studio asks the engine for status and audio. */
const POLL_INTERVAL_MS = 1000;

/**
 * The Studio's only screen.
 *
 * Polling is deliberately boring: one interval, and a single in-flight guard so
 * a slow engine can never cause overlapping requests or an unbounded queue of
 * pending tasks. The interval and the guard are both cleared on unmount.
 */
export default function App(): React.JSX.Element {
  const [view, setView] = useState<ViewModel>(initialView);
  const [audio, setAudio] = useState<AudioViewModel>(initialAudioView);
  const [endpoint, setEndpoint] = useState<string | null>(null);
  const inFlight = useRef(false);

  const poll = useCallback(async (cancelled: () => boolean) => {
    if (inFlight.current) {
      return;
    }
    inFlight.current = true;
    try {
      const raw = await fetchEngineStatus();
      if (!cancelled()) {
        setView(describeEngineView(raw));
      }
    } catch (error) {
      if (!cancelled()) {
        setView(describeInvokeError(error));
      }
    }

    // Audio is a separate request so a missing audio subsystem cannot make the
    // connection state look broken.
    try {
      const raw = await fetchAudioTelemetry();
      if (!cancelled()) {
        setAudio(describeAudio(raw));
      }
    } catch {
      if (!cancelled()) {
        setAudio({
          ...initialAudioView(),
          kind: 'unavailable',
          label: 'Audio unavailable',
          detail: 'the engine did not answer the audio telemetry request',
        });
      }
    } finally {
      inFlight.current = false;
    }
  }, []);

  useEffect(() => {
    let isCancelled = false;
    const cancelled = (): boolean => isCancelled;

    void poll(cancelled);
    const interval = window.setInterval(() => {
      void poll(cancelled);
    }, POLL_INTERVAL_MS);

    void fetchEndpoint()
      .then((value) => {
        if (!isCancelled) {
          setEndpoint(value);
        }
      })
      .catch(() => {
        // The endpoint is diagnostic only; failing to read it must not change
        // the connection state shown to the user.
      });

    return () => {
      isCancelled = true;
      window.clearInterval(interval);
      // Release the pipe handle; the engine keeps running.
      void disconnectFromEngine().catch(() => undefined);
    };
  }, [poll]);

  const status = view.status;
  const audioConnected = view.kind === 'connected';

  return (
    <main className="studio">
      <header className="studio__header">
        <h1 className="studio__title">DiscoDesk Studio</h1>
        <p className="studio__subtitle">
          Control panel. The engine keeps running when this window closes.
        </p>
      </header>

      <section
        className="status"
        data-kind={view.kind}
        data-reachable={view.reachable ? 'true' : 'false'}
        aria-live="polite"
      >
        <div className="status__headline">
          <span className="status__glyph" aria-hidden="true">
            {view.glyph}
          </span>
          <h2 className="status__label">{view.label}</h2>
        </div>
        <p className="status__explanation">{view.explanation}</p>
        {view.detail ? <p className="status__detail">{view.detail}</p> : null}
      </section>

      <section className="audio" data-kind={audio.kind} aria-label="Audio capture">
        <div className="audio__head">
          <h2 className="audio__title">Audio</h2>
          <span className="audio__state">{audio.label}</span>
        </div>

        {audio.device ? (
          <p className="audio__device">
            {audio.device}
            {audio.format ? <span className="audio__format"> · {audio.format}</span> : null}
          </p>
        ) : null}

        {audio.detail ? <p className="audio__detail">{audio.detail}</p> : null}

        <ul className="bands">
          {audio.bands.map((band) => (
            <li className="band" key={band.key}>
              <span className="band__label">{band.label}</span>
              <span
                className="band__track"
                role="meter"
                aria-label={`${band.label} level`}
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={Math.round(band.envelope * 100)}
              >
                <span
                  className="band__fill"
                  style={{ width: `${Math.round(band.envelope * 100)}%` }}
                />
              </span>
              <span className="band__value mono">{band.envelope.toFixed(2)}</span>
            </li>
          ))}
        </ul>

        <dl className="details__list audio__metrics">
          <div className="details__row">
            <dt>Level</dt>
            <dd>
              {audio.levelDbfs === undefined
                ? 'no frames yet'
                : `${audio.levelDbfs.toFixed(1)} dBFS`}
              {audio.onset ? <span className="onset"> · onset</span> : null}
            </dd>
          </div>
          <div className="details__row">
            <dt>Packets / dropped</dt>
            <dd>
              {audio.packets ?? 0} / {audio.dropped ?? 0}
            </dd>
          </div>
          <div className="details__row">
            <dt>DSP per hop</dt>
            <dd>
              {audio.dspLastMicros ?? 0} µs (worst {audio.dspMaxMicros ?? 0} µs)
            </dd>
          </div>
          <div className="details__row">
            <dt>Frames / hop rate</dt>
            <dd>
              {audio.dspFrames ?? 0} /{' '}
              {audio.hopHz === undefined ? '—' : `${audio.hopHz.toFixed(2)} Hz`}
            </dd>
          </div>
        </dl>

        {!audioConnected ? (
          <p className="audio__hint">
            Audio telemetry needs the engine. It will appear when the connection is restored.
          </p>
        ) : null}
      </section>

      <section className="details" aria-label="Engine details">
        <dl className="details__list">
          <div className="details__row">
            <dt>Endpoint</dt>
            <dd className="mono">{endpoint ?? 'unknown'}</dd>
          </div>
          {status ? (
            <>
              <div className="details__row">
                <dt>Engine state</dt>
                <dd>{status.state}</dd>
              </div>
              <div className="details__row">
                <dt>Engine version</dt>
                <dd>{status.engineVersion ?? 'not reported'}</dd>
              </div>
              <div className="details__row">
                <dt>Protocol</dt>
                <dd>{status.protocolVersion}</dd>
              </div>
              <div className="details__row">
                <dt>Uptime</dt>
                <dd>{formatUptime(status.uptimeMs)}</dd>
              </div>
              <div className="details__row">
                <dt>Wallpaper surfaces</dt>
                <dd>{status.surfaceCount}</dd>
              </div>
            </>
          ) : (
            <div className="details__row">
              <dt>Engine state</dt>
              <dd>unavailable while disconnected</dd>
            </div>
          )}
        </dl>
      </section>

      <footer className="studio__footer">
        <p>
          This window never captures audio, never renders the wallpaper and never stops the
          engine.
        </p>
      </footer>
    </main>
  );
}
