//! `vibe-scan` — the HAL BOM ceremony's field tool. `robotd` trusts the compiled joint
//! table because the alpha board is baked into release binaries, and it *assumes* the ids
//! on the bus match that table. A scan is the exact opposite exercise: it finds out which
//! ids actually answer, **with no prior**. That is why this bin does **not** reuse
//! `JOINT_IDS` or `DynamixelIo::open` — both would smuggle the compiled table back in as a
//! bias. It opens the bus and pings `0..=252`, nothing more; pass `--manifest` only when
//! you want the answers checked against a descriptor, which is a *later* step of the
//! ceremony, not its premise. See `docs/design/descriptor-bom-checklist.md` §3.2.
//!
//! Dependency-thin on purpose: no `clap`, no `vibe-ipc-proto`. The ceremony's field tool
//! compiles on the board's cross-target, and the *answers* — not the compiled table — are
//! its payload.

use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let port = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .map(|s| s.as_str())
        .unwrap_or("/dev/ttyS2");
    let baud: u32 = 1_000_000;

    // Open the bus ourselves — not through DynamixelIo::open, which would smuggle the
    // compiled joint table back in.
    let serial = match serialport::new(port, baud)
        .timeout(Duration::from_millis(10))
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
    for id in 0..=252u8 {
        match ctl.ping(id) {
            Ok(true) => found.push(id),
            Ok(false) => {}
            Err(e) => eprintln!("warning: ping {id}: {e}"),
        }
    }

    println!("scan of {port} at {baud} baud: {n} id(s) answer", n = found.len());
    for id in &found {
        println!("  id {id}");
    }
}
