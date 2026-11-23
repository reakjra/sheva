use crc32fast::Hasher;

pub const FRAMES_PER_BLOCK: usize = 32;
pub const BLOCK_LEN: usize = FRAMES_PER_BLOCK * 2;
pub const BT_INPUT_MAX: usize = 128;
pub const FEATURE_MAX: usize = 64;

pub type Block = [u8; BLOCK_LEN];

const CRC_LEN: usize = 4;
const BT_OUTPUT_SEED: u8 = 0xA2;
const BT_FEATURE_SEED: u8 = 0xA3;

const BT_INPUT_ID: u8 = 0x31;
const BT_STATE_ID: u8 = 0x31;
const BT_STATE_LEN: usize = 78;
const BT_HAPTICS_ID: u8 = 0x32;
const BT_HAPTICS_LEN: usize = 142;
const USB_INPUT_ID: u8 = 0x01;
const USB_INPUT_LEN: usize = 64;
const USB_STATE_ID: u8 = 0x02;
const STATE_LEN: usize = 47;
const PAIRING_ID: u8 = 0x09;
const AUDIO_BUFFER_LENGTH: u8 = 48;

pub struct BlockBuilder {
    block: Block,
    filled: usize,
}

impl BlockBuilder {
    pub fn new() -> Self {
        Self {
            block: [0; BLOCK_LEN],
            filled: 0,
        }
    }

    pub fn push(&mut self, frame: [f32; 2]) -> Option<Block> {
        for sample in frame {
            let sample = (sample * 127.0).round().clamp(-128.0, 127.0) as i8;
            self.block[self.filled] = sample.cast_unsigned();
            self.filled += 1;
        }
        if self.filled < BLOCK_LEN {
            return None;
        }
        self.filled = 0;
        Some(self.block)
    }
}

pub struct BtOutput {
    seq: u8,
    packet: u8,
}

impl BtOutput {
    pub fn new() -> Self {
        Self { seq: 0, packet: 0 }
    }

    pub fn haptics(&mut self, block: &Block) -> [u8; BT_HAPTICS_LEN] {
        let mut report = [0; BT_HAPTICS_LEN];
        report[..10].copy_from_slice(&[
            BT_HAPTICS_ID,
            self.next_seq(),
            0x91,
            4,
            0b0110_0110,
            AUDIO_BUFFER_LENGTH,
            AUDIO_BUFFER_LENGTH,
            self.packet,
            0x92,
            BLOCK_LEN as u8,
        ]);
        report[10..10 + BLOCK_LEN].copy_from_slice(block);
        self.packet = self.packet.wrapping_add(1);
        seal(&mut report);
        report
    }

    pub fn state(&mut self, usb: &[u8]) -> Option<[u8; BT_STATE_LEN]> {
        let state = usb.strip_prefix(&[USB_STATE_ID])?;
        let state = &state[..state.len().min(STATE_LEN)];
        let mut report = [0; BT_STATE_LEN];
        report[..3].copy_from_slice(&[BT_STATE_ID, self.next_seq(), 0x10]);
        report[3..3 + state.len()].copy_from_slice(state);
        seal(&mut report);
        Some(report)
    }

    fn next_seq(&mut self) -> u8 {
        let tag = self.seq << 4;
        self.seq = (self.seq + 1) & 0x0F;
        tag
    }
}

pub fn usb_input(bt: &[u8]) -> Option<[u8; USB_INPUT_LEN]> {
    let payload = bt.strip_prefix(&[BT_INPUT_ID])?.get(1..USB_INPUT_LEN)?;
    let mut report = [0; USB_INPUT_LEN];
    report[0] = USB_INPUT_ID;
    report[1..].copy_from_slice(payload);
    Some(report)
}

pub fn usb_feature(mut report: Vec<u8>) -> Option<Vec<u8>> {
    let body = report.len().checked_sub(CRC_LEN)?;
    if crc32(BT_FEATURE_SEED, &report[..body]).to_le_bytes() != report[body..] {
        return None;
    }
    report[body..].fill(0);
    // hid-playstation refuses a second pad with the real one's MAC
    if report.first() == Some(&PAIRING_ID) {
        report[1] ^= 0x02;
    }
    Some(report)
}

fn seal(report: &mut [u8]) {
    let body = report.len() - CRC_LEN;
    let crc = crc32(BT_OUTPUT_SEED, &report[..body]);
    report[body..].copy_from_slice(&crc.to_le_bytes());
}

fn crc32(seed: u8, bytes: &[u8]) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(&[seed]);
    hasher.update(bytes);
    hasher.finalize()
}
