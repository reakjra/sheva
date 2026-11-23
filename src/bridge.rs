use std::fs::{self, Permissions};
use std::io::{ErrorKind, Read};
use std::os::fd::AsFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
use nix::unistd::{Group, chown};

use crate::dualsense;
use crate::hidraw::Hidraw;
use crate::report::{self, BT_INPUT_MAX, BtOutput, FEATURE_MAX};
use crate::uhid::{Event, Uhid};

const SOCKET_MODE: u32 = 0o660;

pub fn run(device: &Path, socket: &Path, group: &str) -> Result<()> {
    let mut pad = Hidraw::open(device)?;
    let mut usb_pad = Uhid::create(&dualsense::USB_PAD)?;
    let listener = bind_socket(socket, group)?;
    let mut client: Option<UnixStream> = None;
    let mut output = BtOutput::new();
    let mut input = [0; BT_INPUT_MAX];
    let mut block = [0; report::BLOCK_LEN];

    loop {
        let [pad_ready, usb_ready, listener_ready, client_ready] = {
            let mut fds = vec![
                PollFd::new(pad.as_fd(), PollFlags::POLLIN),
                PollFd::new(usb_pad.as_fd(), PollFlags::POLLIN),
                PollFd::new(listener.as_fd(), PollFlags::POLLIN),
            ];
            if let Some(client) = &client {
                fds.push(PollFd::new(client.as_fd(), PollFlags::POLLIN));
            }
            poll(&mut fds, PollTimeout::NONE).context("polling")?;
            let mut ready = [false; 4];
            for (slot, fd) in ready.iter_mut().zip(&fds) {
                *slot = fd.revents().is_some_and(|events| !events.is_empty());
            }
            ready
        };

        if pad_ready {
            let Ok(bt) = pad.read_report(&mut input) else {
                return Ok(());
            };
            if let Some(usb) = report::usb_input(bt) {
                usb_pad.input(&usb)?;
            }
        }

        if usb_ready {
            match usb_pad.read_event()? {
                Event::Output(usb) => {
                    if let Some(bt) = output.state(&usb) {
                        pad.write_report(&bt)?;
                    }
                }
                Event::GetReport { request, report } => {
                    let feature = pad
                        .feature(report, FEATURE_MAX)
                        .ok()
                        .and_then(report::usb_feature);
                    usb_pad.reply_get_report(request, feature.as_deref())?;
                }
                Event::SetReport { request } => usb_pad.reply_set_report(request)?,
                Event::Other => {}
            }
        }

        if listener_ready {
            client = Some(listener.accept().context("accepting a sink")?.0);
        }

        if client_ready {
            let alive = client
                .as_mut()
                .is_some_and(|client| client.read_exact(&mut block).is_ok());
            if alive {
                pad.write_report(&output.haptics(&block))?;
            } else {
                client = None;
            }
        }
    }
}

fn bind_socket(path: &Path, group: &str) -> Result<UnixListener> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    match fs::remove_file(path) {
        Err(err) if err.kind() != ErrorKind::NotFound => {
            return Err(err).with_context(|| format!("removing stale {}", path.display()));
        }
        _ => {}
    }
    let listener =
        UnixListener::bind(path).with_context(|| format!("binding {}", path.display()))?;
    let gid = Group::from_name(group)?
        .ok_or_else(|| anyhow!("no group named {group}"))?
        .gid;
    chown(path, None, Some(gid)).with_context(|| format!("chowning {}", path.display()))?;
    fs::set_permissions(path, Permissions::from_mode(SOCKET_MODE))?;
    Ok(listener)
}
