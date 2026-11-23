mod bridge;
mod capture;
mod decimate;
mod dualsense;
mod hidraw;
mod report;
mod sink;
mod switch;
mod uhid;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

const SOCKET: &str = "/run/sheva/haptics.sock";

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    mode: Mode,

    /// socket the bridge listens on and the sink connects to
    #[arg(long, global = true, default_value = SOCKET)]
    socket: PathBuf,
}

#[derive(Subcommand)]
enum Mode {
    /// present a bluetooth DualSense as a USB one (root)
    Bridge {
        /// hidraw node of the bluetooth DualSense (like /dev/hidraw8)
        device: PathBuf,

        /// group allowed to connect to the socket
        #[arg(long, default_value = "input")]
        group: String,
    },
    /// expose the DualSense haptics sink and stream it to the bridge (user)
    Sink,
    /// bridge bluetooth DualSenses again (root)
    Enable,
    /// leave bluetooth DualSenses alone until enabled again (root)
    Disable,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.mode {
        Mode::Bridge { device, group } => bridge::run(&device, &cli.socket, &group),
        Mode::Sink => capture::run(&cli.socket),
        Mode::Enable => switch::set(true),
        Mode::Disable => switch::set(false),
    }
}
