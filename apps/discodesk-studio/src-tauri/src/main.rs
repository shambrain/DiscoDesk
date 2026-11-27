//! Studio entry point.
//!
//! Two modes:
//!
//! * default — opens the Studio window.
//! * `--probe [--endpoint <path>]` — performs one headless status round-trip,
//!   prints the result as JSON and exits. Used by `scripts/ipc-smoke.ps1` to
//!   prove the real Studio binary can talk to the real engine binary.
//!
//! The console subsystem is intentionally left attached (no
//! `windows_subsystem = "windows"`) so `--probe` can write to stdout and so
//! diagnostics are visible during development. Hiding the console for the
//! windowed mode is a Studio polish item, not a Gate 0 requirement, and it must
//! not be done by breaking `--probe`.

fn usage() -> String {
    concat!(
        "discodesk-studio ",
        env!("CARGO_PKG_VERSION"),
        "\n\n",
        "USAGE:\n",
        "    discodesk-studio                 Open the Studio window\n",
        "    discodesk-studio --probe         One status round-trip, JSON on stdout\n",
        "    discodesk-studio --endpoint <p>  Use an explicit engine endpoint\n",
        "    discodesk-studio --help          Print this help\n"
    )
    .to_owned()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print!("{}", usage());
        std::process::exit(0);
    }

    let endpoint = match args.iter().position(|arg| arg == "--endpoint") {
        Some(index) => match args.get(index + 1) {
            Some(value) => Some(value.clone()),
            None => {
                eprintln!("--endpoint requires a path argument");
                std::process::exit(2);
            }
        },
        None => None,
    };

    // Reject unknown arguments rather than silently ignoring them: a typo in a
    // script must fail loudly.
    for arg in &args {
        let is_known = arg == "--probe" || arg == "--endpoint" || endpoint.as_deref() == Some(arg);
        if !is_known {
            eprintln!("unrecognised argument: {arg}\n\n{}", usage());
            std::process::exit(2);
        }
    }

    if args.iter().any(|arg| arg == "--probe") {
        let (code, rendered) = discodesk_studio::probe(endpoint);
        println!("{rendered}");
        std::process::exit(code);
    }

    discodesk_studio::run();
}
