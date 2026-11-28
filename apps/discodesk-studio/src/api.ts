/**
 * The only place the Studio talks to Tauri.
 *
 * Kept separate from `connection.ts` so the translation logic can be tested
 * without a Tauri runtime.
 */
import { invoke } from '@tauri-apps/api/core';

/** Asks the Rust side for the current engine view. */
export async function fetchEngineStatus(): Promise<unknown> {
  return invoke<unknown>('engine_status');
}

/** Asks the Rust side for live audio telemetry. */
export async function fetchAudioTelemetry(): Promise<unknown> {
  return invoke<unknown>('audio_telemetry');
}

/** Asks the Rust side for the endpoint it is using (diagnostics only). */
export async function fetchEndpoint(): Promise<string> {
  return invoke<string>('engine_endpoint');
}

/**
 * Releases this client's engine connection without affecting the engine.
 *
 * Called when the Studio window unloads.
 */
export async function disconnectFromEngine(): Promise<void> {
  await invoke<void>('engine_disconnect');
}
