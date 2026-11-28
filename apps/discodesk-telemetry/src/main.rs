//! Audio telemetry CLI.
//!
//! Reads live capture and DSP state **from the engine over the same versioned
//! IPC contract the Studio uses**. It never opens WASAPI itself: a second
//! capture client would compete with the engine, and the point of the engine is
//! that it owns the audio device.
//!
//! ```text
//! discodesk-telemetry                      one human-readable snapshot
//! discodesk-telemetry --watch              continuous human display
//! discodesk-telemetry --json --interval 100  machine-readable, 10 Hz
//! discodesk-telemetry --count 20 --json    bounded, for scripts and CI
//! ```

#![forbid(unsafe_code)]

use std::process::ExitCode;
use std::time::{Duration, Instant};

use clap::Parser;
use dd_ipc::protocol::{PROTOCOL_VERSION, RequestEnvelope, ResponseEnvelope};
use dd_ipc::{Frame, IpcError};
use serde_json::Value;

/// The band keys, in output order, for the human display.
const BAND_KEYS: [&str; 6] = [
    "subBass",
    "punch",
    "warmth",
    "presence",
    "brilliance",
    "air",
];

#[derive(Debug, Parser)]
#[command(
    name = "discodesk-telemetry",
    version,
    about = "Read live audio telemetry from a running DiscoDesk engine",
    disable_help_subcommand = true
)]
struct Cli {
    /// Print one JSON object per sample instead of a human display.
    #[arg(long)]
    json: bool,

    /// Keep sampling until interrupted.
    #[arg(long)]
    watch: bool,

    /// Milliseconds between samples.
    #[arg(long, value_name = "MS", default_value_t = 500)]
    interval: u64,

    /// Stop after this many samples. Overrides --watch.
    #[arg(long, value_name = "N")]
    count: Option<u64>,

    /// Explicit engine endpoint. Defaults to the shared derivation.
    #[arg(long, value_name = "PATH")]
    endpoint: Option<String>,

    /// Ask the engine to start audio capture before sampling.
    #[arg(long)]
    start: bool,

    /// Ask the engine to stop audio capture before sampling.
    #[arg(long)]
    stop: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let endpoint = cli
        .endpoint
        .clone()
        .unwrap_or_else(dd_ipc::default_endpoint_path);

    match run(&cli, &endpoint) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("discodesk-telemetry: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli, endpoint: &str) -> Result<ExitCode, String> {
    let mut transport = dd_ipc::connect(endpoint).map_err(|err| match err {
        IpcError::EndpointUnavailable(detail) => {
            format!("the engine is not running, or its endpoint is unavailable: {detail}")
        }
        other => format!("could not connect to the engine: {other}"),
    })?;

    if cli.start {
        let _ = request(transport.as_mut(), 1, "StartAudio");
    }
    if cli.stop {
        let _ = request(transport.as_mut(), 2, "StopAudio");
    }

    let limit = cli.count.unwrap_or(if cli.watch { u64::MAX } else { 1 });
    let interval = Duration::from_millis(cli.interval.max(10));
    let mut printed = 0_u64;
    let mut request_id = 10_u64;
    let deadline = Instant::now() + Duration::from_secs(60 * 60 * 24);

    while printed < limit && Instant::now() < deadline {
        let response = match request(transport.as_mut(), request_id, "GetAudioTelemetry") {
            Ok(response) => {
                request_id = request_id.wrapping_add(1).max(1);
                response
            }
            Err(err) => return Err(format!("telemetry request failed: {err}")),
        };

        if cli.json {
            println!("{}", compact(&response));
        } else {
            print_human(&response);
        }
        printed += 1;

        if printed < limit {
            std::thread::sleep(interval);
        }
    }

    Ok(ExitCode::SUCCESS)
}

/// Sends one command and returns the response envelope.
fn request(
    transport: &mut dyn dd_ipc::Transport,
    request_id: u64,
    command_name: &str,
) -> Result<ResponseEnvelope, String> {
    let command = dd_ipc::Command::from_name(command_name)
        .map_err(|err| format!("unknown command {command_name}: {err}"))?;
    let envelope = RequestEnvelope::new(request_id, command);
    let payload =
        serde_json::to_vec(&envelope).map_err(|err| format!("could not encode request: {err}"))?;

    transport
        .send_frame(&Frame {
            version: PROTOCOL_VERSION,
            payload,
        })
        .map_err(|err| err.to_string())?;

    let frame = transport.recv_frame().map_err(|err| err.to_string())?;
    let response: ResponseEnvelope =
        serde_json::from_slice(&frame.payload).map_err(|err| format!("bad response: {err}"))?;

    if let Some(error) = &response.error {
        return Err(format!("engine error {}: {}", error.code, error.message));
    }
    Ok(response)
}

/// Renders the response as one JSON line.
fn compact(response: &ResponseEnvelope) -> String {
    response
        .result
        .as_ref()
        .map(|value| value.to_string())
        .unwrap_or_else(|| "{}".to_owned())
}

/// Renders the response as a human display.
fn print_human(response: &ResponseEnvelope) {
    let empty = Value::Null;
    let root = response.result.as_ref().unwrap_or(&empty);
    let audio = root.get("audio").unwrap_or(&empty);

    let state = text(audio, "state").unwrap_or_else(|| "unknown".to_owned());
    let backend = text(audio, "backend").unwrap_or_else(|| "-".to_owned());
    let device = text(audio, "deviceName").unwrap_or_else(|| "(none)".to_owned());
    let rate = number(audio, "sampleRate").unwrap_or(0.0);
    let channels = number(audio, "channels").unwrap_or(0.0);
    let encoding = text(audio, "encoding").unwrap_or_else(|| "-".to_owned());

    println!();
    println!(
        "engine state : {}",
        text(root, "engineState").unwrap_or_default()
    );
    println!("capture      : {state}  ({backend})");
    println!("device       : {device}");
    println!("format       : {rate:.0} Hz, {channels:.0} ch, {encoding}");

    if let Some(frame) = audio.get("frame").filter(|value| !value.is_null()) {
        let loudness = number(frame, "loudnessDbfs").unwrap_or(-120.0);
        let rms = number(frame, "levelRms").unwrap_or(0.0);
        let peak = number(frame, "levelPeak").unwrap_or(0.0);
        let flux = number(frame, "spectralFlux").unwrap_or(0.0);
        let onset = frame.get("onset").and_then(Value::as_bool).unwrap_or(false);

        println!("level        : {loudness:.1} dBFS   rms {rms:.3}   peak {peak:.3}");
        println!();

        let envelopes = frame.get("bandEnvelopes");
        let raw = frame.get("bands");
        for (index, key) in BAND_KEYS.iter().enumerate() {
            let value = envelopes
                .and_then(|array| array.get(index))
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let instantaneous = raw
                .and_then(|array| array.get(index))
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            println!(
                "  {key:<11} {} {:>5.2}  (raw {:.2})",
                bar(value),
                value,
                instantaneous
            );
        }

        println!();
        println!(
            "flux         : {flux:.2}{}",
            if onset { "   onset!" } else { "" }
        );
    } else {
        println!("frame        : none yet (is anything playing?)");
    }

    if let Some(stats) = audio.get("stats") {
        println!(
            "packets      : {}  silent {}  dropped {}  restarts {}  device changes {}",
            number(stats, "packets").unwrap_or(0.0),
            number(stats, "silentPackets").unwrap_or(0.0),
            number(stats, "droppedSamples").unwrap_or(0.0),
            number(stats, "restarts").unwrap_or(0.0),
            number(stats, "deviceChanges").unwrap_or(0.0)
        );
    }

    println!(
        "buffer       : {}/{} samples",
        number(audio, "bufferFill").unwrap_or(0.0),
        number(audio, "bufferCapacity").unwrap_or(0.0)
    );
    println!(
        "dsp          : {} us last, {} us worst, {} frames, hop {:.2} Hz",
        number(audio, "dspLastMicros").unwrap_or(0.0),
        number(audio, "dspMaxMicros").unwrap_or(0.0),
        number(audio, "dspFrames").unwrap_or(0.0),
        number(audio, "hopRateHz").unwrap_or(0.0)
    );
    if let Some(age) = audio.get("frameAgeMs").and_then(Value::as_u64) {
        println!("frame age    : {age} ms");
    }
    if let Some(error) = audio
        .get("stats")
        .and_then(|stats| text(stats, "lastError"))
    {
        println!("last error   : {error}");
    }
}

/// A 24-cell bar.
fn bar(value: f64) -> String {
    let clamped = value.clamp(0.0, 1.0);
    let filled = (clamped * 24.0).round() as usize;
    format!("[{}{}]", "#".repeat(filled), "-".repeat(24 - filled))
}

fn text(value: &Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(str::to_owned)
}

fn number(value: &Value, key: &str) -> Option<f64> {
    value.get(key)?.as_f64()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bars_are_fixed_width_and_clamped() {
        assert_eq!(bar(0.0), format!("[{}]", "-".repeat(24)));
        assert_eq!(bar(1.0), format!("[{}]", "#".repeat(24)));
        assert_eq!(bar(5.0), format!("[{}]", "#".repeat(24)));
        assert_eq!(bar(-1.0), format!("[{}]", "-".repeat(24)));
        assert_eq!(bar(0.5).len(), 26);
    }

    #[test]
    fn missing_fields_are_tolerated() {
        let value = serde_json::json!({ "state": "Streaming" });
        assert_eq!(text(&value, "state").as_deref(), Some("Streaming"));
        assert_eq!(text(&value, "absent"), None);
        assert_eq!(number(&value, "absent"), None);
    }
}
