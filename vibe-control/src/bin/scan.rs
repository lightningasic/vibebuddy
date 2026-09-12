//! `vibe-scan` — the HAL BOM ceremony's field tool. `robotd` trusts the compiled joint
//! table because the alpha board is baked into release binaries, and it *assumes* the ids
//! on the bus match that table. A scan is the exact opposite exercise: it finds out which
//! ids actually answer, **with no prior**. That is why this bin does **not** reuse
//! `JOINT_IDS` or `DynamixelIo::open` — both would smuggle the compiled table back in as a
//! bias. It opens the bus and pings `0..=252`, nothing more; pass `--manifest` only when
//! you want the answers checked against a descriptor, which is a *later* step of the
//! ceremony, not its premise. See `docs/design/descriptor-bom-checklist.md` §3.2–3.3.
//!
//! Dependency-thin on purpose: no `clap`, no `vibe-ipc-proto`. The ceremony's field tool
//! compiles on the board's cross-target, and the *answers* — not the compiled table — are
//! its payload.

use std::path::PathBuf;
use std::time::Duration;

const DEFAULT_PORT: &str = "/dev/ttyS2";
const DEFAULT_BAUD: u32 = 1_000_000;
const PING_TIMEOUT_MS: u64 = 10;
const MAX_ID: u8 = 252;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // --- parse flags (hand-rolled: no clap) ---
    let mut port: Option<String> = None;
    let mut baud = DEFAULT_BAUD;
    let mut manifest: Option<PathBuf> = None;
    let mut show_help = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => show_help = true,
            "--baud" => {
                i += 1;
                match args.get(i).and_then(|v| v.parse::<u32>().ok()) {
                    Some(b) => baud = b,
                    None => {
                        eprintln!("error: --baud requires a number");
                        std::process::exit(1);
                    }
                }
            }
            "--manifest" => {
                i += 1;
                manifest = args.get(i).map(PathBuf::from);
            }
            other if other.starts_with('-') => {
                eprintln!("error: unknown flag {other}");
                eprintln!("try --help");
                std::process::exit(1);
            }
            s => port = Some(s.to_owned()),
        }
        i += 1;
    }

    if show_help {
        println!("vibe-scan — HAL BOM ceremony field tool");
        println!();
        println!("USAGE");
        println!("  vibe-scan [OPTIONS] [PORT]");
        println!();
        println!("OPTIONS");
        println!("  --baud <N>            Override baud rate (default: {DEFAULT_BAUD})");
        println!("  --manifest <PATH>     Cross-check found ids against a hardware descriptor");
        println!("  -h, --help            Print this help");
        println!();
        println!("ARGUMENTS");
        println!("  PORT                  Serial port path (default: {DEFAULT_PORT})");
        println!();
        println!("EXAMPLES");
        println!("  vibe-scan                            # scan default port at 1 Mbps");
        println!("  vibe-scan /dev/ttyUSB0               # scan a specific port");
        println!("  vibe-scan --baud 115200 /dev/ttyS0   # override baud");
        println!("  vibe-scan --manifest cat.yaml        # scan + cross-check vs descriptor");
        return;
    }

    let port = port.unwrap_or_else(|| DEFAULT_PORT.to_owned());

    // --- open the bus ourselves — not through DynamixelIo::open, which would smuggle the
    // compiled joint table back in ---
    let serial = match serialport::new(&port, baud)
        .timeout(Duration::from_millis(PING_TIMEOUT_MS))
        .open()
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: cannot open bus {port}: {e}");
            std::process::exit(1);
        }
    };

    let mut ctl = rustypot::servo::dynamixel::xl330::Xl330Controller::new()
        .with_serial_port(serial)
        .with_protocol_v2();

    let mut found = Vec::new();
    for id in 0..=MAX_ID {
        match ctl.ping(id) {
            Ok(true) => found.push(id),
            Ok(false) => {}
            Err(e) => eprintln!("warning: ping {id}: {e}"),
        }
    }

    println!(
        "scan of {port} at {baud} baud: {n} id(s) answer",
        n = found.len()
    );
    for id in &found {
        println!("  id {id}");
    }

    // --- §3.3 cross-check against a descriptor (later ceremony step, not the premise) ---
    if let Some(path) = &manifest {
        match vibe_hal::descriptor::HardwareDescriptor::load(path) {
            Ok(desc) => {
                let mut listed = Vec::new();
                for j in &desc.joints {
                    listed.push(j.id);
                }
                listed.push(desc.imu.id);

                let found_set: std::collections::HashSet<u8> = found.iter().copied().collect();

                let mut missing = Vec::new();
                for id in &listed {
                    if !found_set.contains(id) {
                        missing.push(*id);
                    }
                }

                if missing.is_empty() {
                    println!(
                        "manifest \"{}\": all {n} listed id(s) answered",
                        desc.name,
                        n = listed.len()
                    );
                } else {
                    for id in &missing {
                        println!("manifest mismatch: id {id} listed but silent");
                    }
                }
            }
            Err(e) => eprintln!("error: cannot read manifest {}: {e}", path.display()),
        }
    }
}
