use std::fs::{File, OpenOptions};
use std::io::{Read, Result as IoResult, Write};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd};
use std::path::Path;

use anyhow::{Context, Result};

nix::ioctl_readwrite_buf!(hidiocgfeature, b'H', 0x07, u8);

pub struct Hidraw(File);

impl Hidraw {
    pub fn open(path: &Path) -> Result<Self> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map(Self)
            .with_context(|| format!("opening {}", path.display()))
    }

    pub fn read_report<'b>(&mut self, buffer: &'b mut [u8]) -> IoResult<&'b [u8]> {
        let len = self.0.read(buffer)?;
        Ok(&buffer[..len])
    }

    pub fn write_report(&mut self, report: &[u8]) -> Result<()> {
        self.0.write_all(report).context("writing a report")
    }

    pub fn feature(&self, id: u8, max_len: usize) -> Result<Vec<u8>> {
        let mut report = vec![0; max_len];
        report[0] = id;
        // SAFETY: the kernel writes at most report.len() bytes into a buffer we own
        let len = unsafe { hidiocgfeature(self.0.as_raw_fd(), &mut report) }
            .with_context(|| format!("reading feature report {id:#04x}"))?;
        report.truncate(len as usize);
        Ok(report)
    }
}

impl AsFd for Hidraw {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}
