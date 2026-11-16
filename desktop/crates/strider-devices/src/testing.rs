//! Test helpers: temp directories and a minimal FIT encoder. Available to other crates' tests
//! through the `test-support` feature.
use std::path::PathBuf;

/// A fresh directory under the system temp dir; the caller removes it.
pub fn temp_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!(
        "strider-devices-{tag}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

/// The FIT CRC-16 (table-driven, as in the SDK).
pub fn fit_crc(data: &[u8]) -> u16 {
    const TABLE: [u16; 16] = [
        0x0000, 0xCC01, 0xD801, 0x1400, 0xF001, 0x3C00, 0x2800, 0xE401, 0xA001, 0x6C00, 0x7800,
        0xB401, 0x5000, 0x9C01, 0x8801, 0x4400,
    ];
    data.iter().fold(0u16, |crc, &byte| {
        let mut crc = crc;
        for nibble in [byte & 0xF, byte >> 4] {
            let tmp = TABLE[(crc & 0xF) as usize];
            crc = (crc >> 4) & 0x0FFF;
            crc = crc ^ tmp ^ TABLE[nibble as usize];
        }
        crc
    })
}

fn semicircles(degrees: f64) -> i32 {
    (degrees * 2_147_483_648.0 / 180.0).round() as i32
}

/// A field in a test message: `(field number, base type, little-endian bytes)`.
pub type Field = (u8, u8, Vec<u8>);

/// One message: a definition followed by one data message, little-endian.
pub fn message(global: u16, local: u8, fields: &[Field]) -> Vec<u8> {
    let mut out = vec![0x40 | local, 0, 0];
    out.extend(global.to_le_bytes());
    out.push(fields.len() as u8);
    for (num, base, data) in fields {
        out.extend([*num, data.len() as u8, *base]);
    }
    out.push(local);
    for (_, _, data) in fields {
        out.extend(data);
    }
    out
}

/// Wrap messages in a 14-byte header and a trailing CRC.
pub fn fit_file(messages: &[u8]) -> Vec<u8> {
    let mut out = vec![14, 0x20];
    out.extend(2000u16.to_le_bytes());
    out.extend((messages.len() as u32).to_le_bytes());
    out.extend(b".FIT");
    out.extend(fit_crc(&out).to_le_bytes());
    out.extend(messages);
    out.extend(fit_crc(&out).to_le_bytes());
    out
}

const SINT32: u8 = 0x85;
const UINT16: u8 = 0x84;
const UINT32: u8 = 0x86;

/// A FIT activity with one `record` message per `(lat, lon, altitude in metres)`.
///
/// Altitude uses the standard encoding: `(metres + 500) * 5` in `altitude` (field 2).
pub fn encode_fit(points: &[(f64, f64, f64)]) -> Vec<u8> {
    let mut body = vec![];
    for (i, &(lat, lon, alt)) in points.iter().enumerate() {
        let alt_raw = ((alt + 500.0) * 5.0).round() as u16;
        body.extend(message(
            20,
            0,
            &[
                (
                    253,
                    UINT32,
                    (1_000_000_000 + i as u32).to_le_bytes().to_vec(),
                ),
                (0, SINT32, semicircles(lat).to_le_bytes().to_vec()),
                (1, SINT32, semicircles(lon).to_le_bytes().to_vec()),
                (2, UINT16, alt_raw.to_le_bytes().to_vec()),
            ],
        ));
    }
    fit_file(&body)
}
