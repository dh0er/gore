use goresave_g1r_codec_host::{
    RuntimeSelftestWorkerRequest, builtin_profiles, handle_ipc_line_with_runtime_worker,
    runtime_selftest_worker_report,
};
use std::io::{self, BufRead, Read};
use std::time::Duration;

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--stdio") if args.next().is_none() => run_stdio(),
        Some("--runtime-selftest-worker") => run_runtime_selftest_worker(args.collect()),
        _ => {
            eprintln!("usage: goresave_g1r_codec_host --stdio");
            std::process::exit(2);
        }
    }
}

fn run_stdio() {
    let profiles = builtin_profiles();
    let runtime_worker_path = match std::env::current_exe() {
        Ok(path) => path,
        Err(err) => {
            eprintln!("current executable path lookup failed: {err}");
            std::process::exit(1);
        }
    };
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        match line {
            Ok(line) if !line.trim().is_empty() => {
                println!(
                    "{}",
                    handle_ipc_line_with_runtime_worker(&line, &profiles, &runtime_worker_path)
                );
            }
            Ok(_) => {}
            Err(err) => {
                eprintln!("stdin read failed: {err}");
                std::process::exit(1);
            }
        }
    }
}

fn run_runtime_selftest_worker(args: Vec<String>) {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--delay-ms" => {
                let Some(value) = iter.next() else {
                    eprintln!("--delay-ms requires a value");
                    std::process::exit(2);
                };
                let delay_ms: u64 = match value.parse() {
                    Ok(value) => value,
                    Err(err) => {
                        eprintln!("invalid --delay-ms value: {err}");
                        std::process::exit(2);
                    }
                };
                std::thread::sleep(Duration::from_millis(delay_ms));
            }
            _ => {
                eprintln!("unknown runtime selftest worker argument: {arg}");
                std::process::exit(2);
            }
        }
    }

    let mut input = String::new();
    if let Err(err) = io::stdin().read_to_string(&mut input) {
        eprintln!("runtime selftest worker request read failed: {err}");
        std::process::exit(1);
    }
    let request = if input.trim().is_empty() {
        None
    } else {
        match serde_json::from_str::<RuntimeSelftestWorkerRequest>(&input) {
            Ok(request) => Some(request),
            Err(err) => {
                eprintln!("runtime selftest worker request JSON invalid: {err}");
                std::process::exit(2);
            }
        }
    };

    let report = match runtime_selftest_worker_report(request) {
        Ok(report) => report,
        Err(err) => {
            eprintln!("runtime selftest worker failed: {err}");
            std::process::exit(1);
        }
    };
    match serde_json::to_string(&report) {
        Ok(report) => println!("{report}"),
        Err(err) => {
            eprintln!("runtime selftest worker report serialization failed: {err}");
            std::process::exit(1);
        }
    }
}
