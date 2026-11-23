use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::{AsFd, BorrowedFd};

use anyhow::{Context, Result, bail};

const DEVICE: &str = "/dev/uhid";
const EVENT_LEN: usize = 4376;
const DATA_MAX: usize = 4096;
const NAME_LEN: usize = 128;
const PHYS_LEN: usize = 64;
const UNIQ_LEN: usize = 64;
const EIO: u16 = 5;

const DESTROY: u32 = 1;
const OUTPUT: u32 = 6;
const GET_REPORT: u32 = 9;
const GET_REPORT_REPLY: u32 = 10;
const CREATE2: u32 = 11;
const INPUT2: u32 = 12;
const SET_REPORT: u32 = 13;
const SET_REPORT_REPLY: u32 = 14;

pub struct DeviceInfo<'a> {
    pub name: &'a str,
    pub phys: &'a str,
    pub bus: u16,
    pub vendor: u32,
    pub product: u32,
    pub version: u32,
    pub descriptor: &'a [u8],
}

pub enum Event {
    Output(Vec<u8>),
    GetReport { request: u32, report: u8 },
    SetReport { request: u32 },
    Other,
}

pub struct Uhid {
    file: File,
    buffer: Box<[u8; EVENT_LEN]>,
}

impl Uhid {
    pub fn create(info: &DeviceInfo) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(DEVICE)
            .with_context(|| format!("opening {DEVICE}"))?;
        let mut uhid = Self {
            file,
            buffer: Box::new([0; EVENT_LEN]),
        };
        uhid.send(
            EventWriter::new(CREATE2)
                .padded(info.name.as_bytes(), NAME_LEN)
                .padded(info.phys.as_bytes(), PHYS_LEN)
                .padded(&[], UNIQ_LEN)
                .u16(info.descriptor.len() as u16)
                .u16(info.bus)
                .u32(info.vendor)
                .u32(info.product)
                .u32(info.version)
                .u32(0)
                .bytes(info.descriptor),
        )?;
        Ok(uhid)
    }

    pub fn input(&mut self, report: &[u8]) -> Result<()> {
        self.send(
            EventWriter::new(INPUT2)
                .u16(report.len() as u16)
                .bytes(report),
        )
    }

    pub fn reply_get_report(&mut self, request: u32, report: Option<&[u8]>) -> Result<()> {
        let data = report.unwrap_or_default();
        self.send(
            EventWriter::new(GET_REPORT_REPLY)
                .u32(request)
                .u16(if report.is_some() { 0 } else { EIO })
                .u16(data.len() as u16)
                .bytes(data),
        )
    }

    pub fn reply_set_report(&mut self, request: u32) -> Result<()> {
        self.send(EventWriter::new(SET_REPORT_REPLY).u32(request).u16(0))
    }

    pub fn read_event(&mut self) -> Result<Event> {
        let len = self
            .file
            .read(&mut self.buffer[..])
            .context("reading uhid")?;
        if len < EVENT_LEN {
            bail!("short uhid event ({len} bytes)");
        }
        let event = &self.buffer[..];
        Ok(match u32_at(event, 0) {
            OUTPUT => {
                let size = usize::from(u16_at(event, 4 + DATA_MAX)).min(DATA_MAX);
                Event::Output(event[4..4 + size].to_vec())
            }
            GET_REPORT => Event::GetReport {
                request: u32_at(event, 4),
                report: event[8],
            },
            SET_REPORT => Event::SetReport {
                request: u32_at(event, 4),
            },
            _ => Event::Other,
        })
    }

    fn send(&mut self, event: &mut EventWriter) -> Result<()> {
        event.0.resize(EVENT_LEN, 0);
        self.file.write_all(&event.0).context("writing uhid")
    }
}

impl Drop for Uhid {
    fn drop(&mut self) {
        let _ = self.send(&mut EventWriter::new(DESTROY));
    }
}

impl AsFd for Uhid {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.file.as_fd()
    }
}

struct EventWriter(Vec<u8>);

impl EventWriter {
    fn new(kind: u32) -> Self {
        let mut bytes = Vec::with_capacity(EVENT_LEN);
        bytes.extend_from_slice(&kind.to_le_bytes());
        Self(bytes)
    }

    fn u16(&mut self, value: u16) -> &mut Self {
        self.bytes(&value.to_le_bytes())
    }

    fn u32(&mut self, value: u32) -> &mut Self {
        self.bytes(&value.to_le_bytes())
    }

    fn bytes(&mut self, bytes: &[u8]) -> &mut Self {
        self.0.extend_from_slice(bytes);
        self
    }

    fn padded(&mut self, bytes: &[u8], len: usize) -> &mut Self {
        let start = self.0.len();
        self.0.extend_from_slice(&bytes[..bytes.len().min(len)]);
        self.0.resize(start + len, 0);
        self
    }
}

fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(field(bytes, offset))
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(field(bytes, offset))
}

fn field<const N: usize>(bytes: &[u8], offset: usize) -> [u8; N] {
    *bytes[offset..]
        .first_chunk()
        .expect("uhid events are fixed size")
}
