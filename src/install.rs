//! `install` / `uninstall`: the binary, a systemd service and a udev rule, all
//! system-wide. One privileged step through sudo.

use std::io::{self, BufRead, IsTerminal, Write};
use std::path::Path;
use std::process::Command;

const BIN: &str = "/usr/local/bin/blackshark-battery";
const UNIT_DIR: &str = "~/.config/systemd/user";
const UNIT: &str = "~/.config/systemd/user/blackshark-battery.service";
const UDEV_RULE: &str = "/etc/udev/rules.d/60-blackshark-battery.rules";

const UNIT_BODY: &str = include_str!("../dist/blackshark-battery.service");
const UDEV_RULE_BODY: &str = include_str!("../dist/60-blackshark-battery.rules");

pub fn install(yes: bool) -> u8 {
    println!("blackshark-battery install\n");
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            println!("cannot locate this binary: {e}");
            return 1;
        }
    };

    println!("This writes, as root:");
    println!("  {BIN} (this binary)");
    println!("  {UDEV_RULE}:\n{}", indent(UDEV_RULE_BODY));
    println!("This writes, as user:");
    println!("  {UNIT}: system service that polls the headset and feeds the virtual battery");
    println!("then enables and starts blackshark-battery.service.\n");
    if !confirm("Apply with sudo?", yes) {
        println!("Skipped.");
        return 1;
    }

    let privileged_script = format!(
        "set -e; \
         install -m755 {exe} {BIN}.new && mv -f {BIN}.new {BIN}; \
         printf '%s' {rule} > {UDEV_RULE}; \
         udevadm control --reload-rules; \
        ",
        exe = sh_quote(&exe.to_string_lossy()),
        rule = sh_quote(UDEV_RULE_BODY),
    );
    if !run_privileged(&privileged_script) {
        println!("Privileged step failed.");
        return 1;
    }

    let script = format!(
        "set -e; \
         mkdir -p {UNIT_DIR}; \
         printf '%s' {unit} > {UNIT}; \
         systemctl --user daemon-reload; \
         systemctl --user enable blackshark-battery.service; \
         systemctl --user restart blackshark-battery.service",
        unit = sh_quote(UNIT_BODY),
    );
    if !run(&script) {
        println!("User step failed.");
        return 1;
    }

    println!("[service] blackshark-battery.service enabled and started");
    println!("\nThe headset shows up in the power applet within a few seconds while it's on.");
    println!("Check: blackshark-battery status, journalctl --user -u blackshark-battery -f");
    0
}

pub fn uninstall(yes: bool) -> u8 {
    println!("blackshark-battery uninstall\n");
    let present: Vec<&str> = [BIN, UDEV_RULE, UNIT].into_iter().filter(|p| Path::new(p).exists()).collect();
    if present.is_empty() {
        println!("Nothing installed.");
        return 0;
    }
    println!("This removes bin and udev, as root:");
    for p in &present[0..2] {
        println!("  {p}");
    }
    println!("This stops service, as user:");
    println!("  {UNIT}");
    if !confirm("Apply with sudo?", yes) {
        println!("Skipped.");
        return 1;
    }
    let priviledged_script = format!(
        "rm -f {BIN} {UDEV_RULE}; \
         systemctl daemon-reload; udevadm control --reload-rules"
    );
    if !run_privileged(&priviledged_script) {
        println!("Privileged step failed.");
        return 1;
    }
    let script = format!(
        "systemctl --user stop lackshark-battery.service 2>/dev/null; \
         systemctl --user disable --now blackshark-battery.service 2>/dev/null; \
         rm -f {UNIT}"
    );
    if !run(&script) {
        println!("User step failed.");
        return 1;
    }

    println!("[remove] done");
    0
}

fn confirm(question: &str, yes: bool) -> bool {
    if yes {
        println!("{question} [y/N] y");
        return true;
    }
    if !io::stdin().is_terminal() {
        println!("{question} [y/N] (not a terminal: no; pass --yes to accept)");
        return false;
    }
    print!("{question} [y/N] ");
    let _ = io::stdout().flush();
    let mut line = String::new();
    let _ = io::stdin().lock().read_line(&mut line);
    matches!(line.trim(), "y" | "Y" | "yes")
}

/// Run a shell snippet as root: directly if we already aren, else sudo.
fn run_privileged(script: &str) -> bool {
    let mut cmd = if unsafe { libc::geteuid() } == 0 {
        Command::new("sh")
    } else {
        let mut c = Command::new("sudo");
        c.arg("sh");
        c
    };
    cmd.args(["-c", script]).status().is_ok_and(|s| s.success())
}

fn run(script: &str) -> bool {
    Command::new("sh").args(["-c", script]).status().is_ok_and(|s| s.success())
}

fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn indent(s: &str) -> String {
    s.lines().map(|l| format!("    {l}\n")).collect()
}
