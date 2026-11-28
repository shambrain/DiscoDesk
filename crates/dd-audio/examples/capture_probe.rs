//! Capture probe: opens loopback capture and prints what the runtime sees.
//!
//! Run with:
//!
//! ```powershell
//! cargo run -p dd-audio --example capture_probe -- 5
//! ```
//!
//! The argument is the duration in seconds. This is developer tooling and the
//! basis of the hardware integration test; it is not the telemetry CLI, which
//! talks to the engine over IPC.

use std::thread;
use std::time::Duration;

use dd_audio::{AudioRuntime, AudioRuntimeConfig};

fn main() {
    let seconds = std::env::args()
        .nth(1)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(3)
        .clamp(1, 60);

    println!("capturing for {seconds}s on the default output device\u{2026}");

    let runtime = AudioRuntime::start(AudioRuntimeConfig::default());
    let deadline = std::time::Instant::now() + Duration::from_secs(seconds);
    let mut last_frames = 0_u64;

    while std::time::Instant::now() < deadline {
        thread::sleep(Duration::from_millis(500));
        let telemetry = runtime.telemetry();
        if telemetry.dsp_frames != last_frames {
            last_frames = telemetry.dsp_frames;
            let bands = telemetry
                .frame
                .map(|frame| {
                    frame
                        .band_envelopes
                        .iter()
                        .map(|value| format!("{value:.2}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_else(|| "-".to_owned());
            println!(
                "{:?} device={:?} {}Hz/{}ch {} packets={} silent={} dropped={} dsp={:?}us worst={}us loud={:?} bands=[{}]",
                telemetry.state,
                telemetry.device_name,
                telemetry.sample_rate.unwrap_or(0),
                telemetry.channels.unwrap_or(0),
                telemetry.encoding.unwrap_or_else(|| "-".into()),
                telemetry.stats.packets,
                telemetry.stats.silent_packets,
                telemetry.stats.dropped_samples,
                telemetry.dsp_last_micros,
                telemetry.dsp_max_micros,
                telemetry.frame.map(|f| f.loudness_dbfs),
                bands,
            );
        }
    }

    let final_state = runtime.telemetry();
    runtime.stop();

    println!("\n--- summary ---");
    println!("state            : {:?}", final_state.state);
    println!("backend          : {:?}", final_state.backend);
    println!("device           : {:?}", final_state.device_name);
    println!(
        "format           : {} Hz, {} ch, {}",
        final_state.sample_rate.unwrap_or(0),
        final_state.channels.unwrap_or(0),
        final_state.encoding.clone().unwrap_or_else(|| "-".into())
    );
    println!("packets          : {}", final_state.stats.packets);
    println!("silent packets   : {}", final_state.stats.silent_packets);
    println!("samples          : {}", final_state.stats.samples);
    println!("dropped samples  : {}", final_state.stats.dropped_samples);
    println!("restarts         : {}", final_state.stats.restarts);
    println!("dsp frames       : {}", final_state.dsp_frames);
    println!("dsp last         : {} us", final_state.dsp_last_micros);
    println!("dsp worst        : {} us", final_state.dsp_max_micros);
    println!(
        "hop rate         : {:.2} Hz",
        final_state.hop_rate_hz.unwrap_or(0.0)
    );
    if let Some(error) = final_state.stats.last_error {
        println!("last error       : {error}");
    }
    if let Some(frame) = final_state.frame {
        println!("last loudness    : {:.1} dBFS", frame.loudness_dbfs);
        println!("last bands       : {:?}", frame.bands);
        println!("last envelopes   : {:?}", frame.band_envelopes);
    }
}
