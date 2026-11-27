import { useCallback, useEffect, useRef, useState } from 'react';
import { disconnectFromEngine, fetchEndpoint, fetchEngineStatus } from './api';
import {
  describeEngineView,
  describeInvokeError,
  formatUptime,
  initialView,
  type ViewModel,
} from './connection';

/** How often the Studio asks the engine for its status. */
const POLL_INTERVAL_MS = 2000;

/**
 * The Studio's only screen.
 *
 * Polling is deliberately boring: one interval, and a single in-flight guard so
 * a slow engine can never cause overlapping requests or an unbounded queue of
 * pending tasks. The interval and the guard are both cleared on unmount.
 */
export default function App(): React.JSX.Element {
  const [view, setView] = useState<ViewModel>(initialView);
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

  return (
    <main className="studio">
      <header className="studio__header">
        <h1 className="studio__title">DiscoDesk Studio</h1>
        <p className="studio__subtitle">Control panel. The engine keeps running when this window closes.</p>
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
          This window never captures audio, never renders the wallpaper and never stops the engine.
        </p>
      </footer>
    </main>
  );
}
