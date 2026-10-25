mod dongle;
mod install;
mod tray;

use std::io;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use dongle::{Battery, Dongle};
use hidapi::HidError;

use crate::tray::HeadsetTrayIcon;

const DEFAULT_INTERVAL: Duration = Duration::from_secs(5);

const USAGE: &str = "\
usage: blackshark-battery <command>

commands:
  run [--interval SECONDS]  poll the headset and publish its battery (default every 5s)
  status                    print the headset battery state once
  install [--yes]           install the binary, systemd service and udev rule (asks for root)
  uninstall [--yes]         remove them again

All commands except install/uninstall need root (the service runs as root).";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["--version" | "-V"] => {
            println!("blackshark-battery {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        ["--help" | "-h" | "help"] => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        ["install", rest @ ..] => {
            return parse_yes(rest).map_or_else(usage, |yes| ExitCode::from(install::install(yes)));
        }
        ["uninstall", rest @ ..] => {
            return parse_yes(rest).map_or_else(usage, |yes| ExitCode::from(install::uninstall(yes)));
        }
        ["status"] => print_status(),
        ["run"] => run(DEFAULT_INTERVAL),
        ["run", "--interval", secs] => match secs.parse::<u64>() {
            Ok(s) if s > 0 => run(Duration::from_secs(s)),
            _ => return usage(),
        },
        _ => return usage(),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn usage() -> ExitCode {
    eprintln!("{USAGE}");
    ExitCode::from(2)
}

fn parse_yes(rest: &[&str]) -> Option<bool> {
    match rest {
        [] => Some(false),
        ["--yes" | "-y"] => Some(true),
        _ => None,
    }
}

fn print_status() -> Result<(), HidError> {
    let Some(mut dongle) = Dongle::open()? else {
        return Err(io::Error::other("dongle 1532:0565 not found").into());
    };
    match dongle.battery()? {
        Some(b) => println!("{}%{}", b.percent, if b.charging { " charging" } else { "" }),
        None => println!("headset not responding"),
    }
    Ok(())
}

fn run(interval: Duration) -> ! {
    let mut last: Option<Battery> = None;
    let mut next_poll = Instant::now();

    let mut tray_icon = HeadsetTrayIcon::new();

    loop {
        let now = Instant::now();
        if now >= next_poll {
            next_poll = now + interval;
            let state = poll_headset();
            dbg!(&state);
            if state != last {
                match state {
                    Some(b) => eprintln!("battery {}% ({}charging)", b.percent, if b.charging { "" } else { "dis" }),
                    None => eprintln!("headset unavailable"),
                }
                last = state;

                match state {
                    Some(b) => tray_icon.change_battery_level(b.percent, Some(b.charging)),
                    None => tray_icon.set_disconnected(),
                }
            }
        }

        let timeout = next_poll.saturating_duration_since(Instant::now());
        println!("Next update in {:?}", &timeout);
        std::thread::sleep(timeout)
    }
}

/// Query the headset. Looks the device up afresh every time, so it follows the headset
/// between the dongle and the charging cable. `None` when it's off or unplugged.
fn poll_headset() -> Option<Battery> {
    let result = Dongle::open().and_then(|d| match d {
        Some(mut d) => d.battery(),
        None => Ok(None),
    });
    result.unwrap_or_else(|e| {
        eprintln!("headset: {e}");
        None
    })
}
