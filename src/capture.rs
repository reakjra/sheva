use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::mpsc::{Receiver, sync_channel};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use pipewire::channel;
use pipewire::context::ContextRc;
use pipewire::main_loop::MainLoopRc;

use crate::decimate::Decimator;
use crate::dualsense;
use crate::report::{Block, BlockBuilder};
use crate::sink::Sink;

const QUEUED_BLOCKS: usize = 8;
const RECONNECT_DELAY: Duration = Duration::from_secs(1);

pub fn run(socket: &Path) -> Result<()> {
    pipewire::init();
    loop {
        match UnixStream::connect(socket) {
            Ok(bridge) => {
                if let Err(err) = serve(bridge) {
                    eprintln!("sheva: {err:#}");
                }
            }
            Err(_) => thread::sleep(RECONNECT_DELAY),
        }
    }
}

fn serve(bridge: UnixStream) -> Result<()> {
    let mainloop = MainLoopRc::new(None)?;
    let context = ContextRc::new(&mainloop, None)?;
    let core = context.connect_rc(None)?;

    let (blocks_tx, blocks_rx) = sync_channel::<Block>(QUEUED_BLOCKS);
    let writer = bridge.try_clone().context("cloning the bridge socket")?;
    thread::spawn(move || forward_blocks(writer, &blocks_rx));

    let (quit_tx, quit_rx) = channel::channel::<()>();
    let mut watcher = bridge;
    thread::spawn(move || {
        let _ = watcher.read(&mut [0]);
        let _ = quit_tx.send(());
    });

    let mut decimator = Decimator::new(dualsense::SINK_FORMAT.rate, dualsense::HAPTICS_RATE);
    let mut blocks = BlockBuilder::new();
    let [left, right] = dualsense::HAPTIC_CHANNELS;
    let _sink = Sink::register(
        core,
        "sheva",
        dualsense::sink_properties(),
        &dualsense::SINK_FORMAT,
        move |frame| {
            let Some(haptics) = decimator.push([frame[left], frame[right]]) else {
                return;
            };
            if let Some(block) = blocks.push(haptics) {
                let _ = blocks_tx.try_send(block);
            }
        },
    )?;

    let quit_loop = mainloop.clone();
    let _quit = quit_rx.attach(mainloop.loop_(), move |()| quit_loop.quit());
    mainloop.run();
    Ok(())
}

fn forward_blocks(mut bridge: UnixStream, blocks: &Receiver<Block>) {
    for block in blocks {
        if bridge.write_all(&block).is_err() {
            return;
        }
    }
}
