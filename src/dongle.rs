use hidapi::{HidApi, HidDevice, HidError};
use std::time::{Duration, Instant};

const REPORT_ID: u8 = 0x02;
const FRAME_LEN: usize = 63;
const REPLY_TIMEOUT: Duration = Duration::from_millis(500);
const VENDOR_PAGE: u16 = 0xff14;

const DEVICES: [(u16, u16, u8); 2] = [
    (0x1532, 0x056e, 0x00), // headset over USB-C; prefer while connected
    (0x1532, 0x0565, 0x80), // dongle
];

pub struct Dongle {
    device: HidDevice,
    domain: u8,
    seq: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Battery {
    pub percent: u8,
    pub charging: bool,
}

impl Dongle {
    pub fn open() -> Result<Option<Self>, HidError> {
        let api = HidApi::new()?;

        // Keep DEVICES ordered cable-first, as in the raw-hidraw version.
        for &(vid, pid, domain) in &DEVICES {
            let Some(info) = api.device_list().find(|info| {
                info.vendor_id() == vid
                    && info.product_id() == pid
                    && info.usage_page() == VENDOR_PAGE
            }) else {
                continue;
            };

            let device = info.open_device(&api)?;
            return Ok(Some(Self { device, domain, seq: 0 }));
        }

        Ok(None)
    }

    fn query(&mut self, cmd: u8) -> Result<Option<u8>, HidError> {
        let seq = 0x60 | (self.seq & 0x1f);
        self.seq = self.seq.wrapping_add(1);

        let report = frame(seq, cmd, self.domain);
        self.device.write(&report)?;

        let deadline = Instant::now() + REPLY_TIMEOUT;
        let mut buf = [0u8; FRAME_LEN + 1];

        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Ok(None);
            }

            // read_timeout takes milliseconds; use at least 1 while time remains.
            let timeout_ms = left.as_millis().clamp(1, i32::MAX as u128) as i32;
            let n = self.device.read_timeout(&mut buf, timeout_ms)?;

            if n == 0 {
                return Ok(None); // timed out
            }

            // hidapi includes the report ID as byte 0 for numbered reports.
            let data = match buf[..n].first() {
                Some(&REPORT_ID) => &buf[1..n],
                _ => &buf[..n],
            };

            // [1] seq, [9] cmd, [10] ack flag, [11] payload count, [12] first payload byte
            if data.len() >= 13
                && data[1] == seq
                && data[9] == cmd
                && data[10] == 0x01
                && data[11] >= 1
            {
                return Ok(Some(data[12]));
            }
        }
    }

    pub fn battery(&mut self) -> Result<Option<Battery>, HidError> {
        let Some(percent) = self.query(0x21)? else {
            return Ok(None);
        };

        if percent > 100 {
            return Ok(None);
        }

        let charging = self
            .query(0x2a)?
            .is_some_and(|value| value > 0);

        Ok(Some(Battery { percent, charging }))
    }
}

// Keep your existing frame() function unchanged.


fn frame(seq: u8, cmd: u8, domain: u8) -> [u8; FRAME_LEN + 1] {
    let mut p = [0u8; FRAME_LEN];
    p[1] = seq;
    p[5] = 0x04; // payload length of a query
    p[8] = domain;
    p[9] = cmd;
    // The dongle drops frames whose XOR checksum (seeded with the report ID) mismatches.
    p[61] = p[..61].iter().fold(REPORT_ID, |x, b| x ^ b);

    let mut out = [0u8; FRAME_LEN + 1];
    out[0] = REPORT_ID;
    out[1..].copy_from_slice(&p);
    out
}
