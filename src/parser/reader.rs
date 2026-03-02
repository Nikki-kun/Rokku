use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Cursor, Read, Seek},
    sync::Arc,
};

use anyhow::{anyhow, Context, Result};
use binrw::{binread, BinRead};

#[binread]
#[br(little, magic = b"\xA3\x95")]
struct PacketHeader {
    msg_id: u8,
}

#[binread]
#[br(little)]
#[derive(Debug, Clone)]
pub struct FmtPacket {
    pub type_id: u8,
    #[allow(dead_code)]
    length: u8,
    #[br(map = |bytes: [u8; 4]| sanitize_str(&bytes))]
    pub name: String,
    #[br(map = |bytes: [u8; 16]| sanitize_str(&bytes))]
    format_str: String,
    #[br(map = |bytes: [u8; 64]| sanitize_str(&bytes))]
    labels: String,
}

fn sanitize_str(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .trim_end_matches('\0')
        .to_string()
}

#[derive(Debug)]
pub enum LogValue {
    Int(i64),
    UInt(u64),
    Float(f32),
    Double(f64),
    Str(String),
}

impl LogValue {
    pub fn to_f64(&self) -> Option<f64> {
        match self {
            LogValue::Int(v) => Some(*v as f64),
            LogValue::UInt(v) => Some(*v as f64),
            LogValue::Float(v) if v.is_finite() => Some(*v as f64),
            LogValue::Double(v) if v.is_finite() => Some(*v),
            _ => None,
        }
    }
}

fn field_length(fmt_char: char) -> Result<u64> {
    match fmt_char {
        'b' | 'B' | 'M' => Ok(1),
        'h' | 'c' | 'H' | 'C' => Ok(2),
        'i' | 'L' | 'I' | 'E' | 'e' | 'f' | 'n' => Ok(4),
        'q' | 'Q' | 'd' => Ok(8),
        'N' => Ok(16),
        'Z' => Ok(64),
        _ => Err(anyhow!("unexpcted char: {}", fmt_char)),
    }
}

fn compute_body_size(format_str: &str) -> Result<usize> {
    let mut total = 0usize;
    for c in format_str.bytes() {
        total += field_length(c as char)? as usize;
    }
    Ok(total)
}

fn parse_value(
    reader: &mut (impl std::io::Read + std::io::Seek),
    fmt_char: char,
) -> Result<LogValue> {
    match fmt_char {
        'b' => Ok(LogValue::Int(i8::read_le(reader)? as i64)),
        'h' | 'c' => Ok(LogValue::Int(i16::read_le(reader)? as i64)),
        'i' | 'L' | 'e' => Ok(LogValue::Int(i32::read_le(reader)? as i64)),
        'q' => Ok(LogValue::Int(i64::read_le(reader)?)),
        'B' | 'M' => Ok(LogValue::UInt(u8::read_le(reader)? as u64)),
        'H' | 'C' => Ok(LogValue::UInt(u16::read_le(reader)? as u64)),
        'I' | 'E' => Ok(LogValue::UInt(u32::read_le(reader)? as u64)),
        'Q' => Ok(LogValue::UInt(u64::read_le(reader)?)),
        'f' => Ok(LogValue::Float(f32::read_le(reader)?)),
        'd' => Ok(LogValue::Double(f64::read_le(reader)?)),
        'n' => {
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf)?;
            Ok(LogValue::Str(sanitize_str(&buf)))
        }
        'N' => {
            let mut buf = [0u8; 16];
            reader.read_exact(&mut buf)?;
            Ok(LogValue::Str(sanitize_str(&buf)))
        }
        'Z' => {
            let mut buf = [0u8; 64];
            reader.read_exact(&mut buf)?;
            Ok(LogValue::Str(sanitize_str(&buf)))
        }
        _ => Err(anyhow!("Unknown format char: {}", fmt_char)),
    }
}

const BUF_SIZE: usize = 2 * 1024 * 1024;
const MSG_BODY_BUF: usize = 256;

pub struct ArduReader {
    filename: String,
    file: Option<BufReader<File>>,
    definitions: HashMap<u8, ArduDefinition>,
    last_timestamp: u64,
    msg_body_buf: Vec<u8>,
}

#[derive(Clone)]
pub struct ArduDefinition {
    pub ardu_fmt: FmtPacket,
    pub labels: Vec<Arc<str>>,
    pub body_size: usize,
    pub name: Arc<str>,
    pub timestamp_field_idx: Option<usize>,
}

pub struct ArduMessage {
    pub type_id: u8,
    pub current_ts: u64,
    pub fields: Vec<(usize, LogValue)>,
}

impl ArduReader {
    pub fn new(filename: &str) -> Self {
        Self {
            filename: filename.to_string(),
            file: None,
            definitions: HashMap::new(),
            last_timestamp: 0,
            msg_body_buf: vec![0u8; MSG_BODY_BUF],
        }
    }

    pub fn get_definition(&self, type_id: &u8) -> Option<&ArduDefinition> {
        self.definitions.get(type_id)
    }

    pub fn read(&mut self) -> Result<ArduFrame> {
        if self.file.is_none() {
            let f = File::open(&self.filename).context("Failed opening file")?;
            self.file = Some(BufReader::with_capacity(BUF_SIZE, f));
        }

        let file = self.file.as_mut().unwrap();

        let header = match PacketHeader::read(file) {
            Ok(h) => h,
            Err(binrw::Error::Io(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Ok(ArduFrame::Eof)
            }
            Err(e) => {
                log::warn!("Unexpected parser error (treated as EOF): {}", e);
                return Ok(ArduFrame::Eof);
            }
        };

        if header.msg_id == 128 {
            let ardu_fmt = FmtPacket::read(file)?;
            let labels: Vec<Arc<str>> = ardu_fmt
                .labels
                .split(',')
                .map(|s| Arc::from(s.trim()))
                .collect();
            let timestamp_field_idx = labels
                .iter()
                .position(|label| label.as_ref() == "TimeUS" || label.as_ref() == "timestamp");

            let body_size = compute_body_size(&ardu_fmt.format_str).unwrap_or(0);

            let definition = ArduDefinition {
                ardu_fmt: ardu_fmt.clone(),
                labels,
                body_size,
                name: Arc::from(ardu_fmt.name.as_str()),
                timestamp_field_idx,
            };

            self.definitions
                .insert(ardu_fmt.type_id, definition.clone());

            return Ok(ArduFrame::ArduDefinition(definition));
        } else if let Some(definition) = self.definitions.get(&header.msg_id) {
            let body_size = definition.body_size;
            if body_size == 0 {
                return Err(anyhow!("Invalid format for msg_id {}", header.msg_id));
            }

            if self.msg_body_buf.len() < body_size {
                self.msg_body_buf.resize(body_size, 0);
            }
            let buf = &mut self.msg_body_buf[..body_size];
            if let Err(e) = file.read_exact(buf) {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    log::warn!(
                        "File is incomplete: expected {} bytes for msg_id {} at EOF",
                        body_size,
                        header.msg_id
                    );
                    return Ok(ArduFrame::Eof);
                }
                return Err(e.into());
            }

            let mut cursor = Cursor::new(buf);
            let mut current_ts = 0u64;
            let mut fields = Vec::with_capacity(definition.labels.len());

            for (idx, c) in definition.ardu_fmt.format_str.bytes().enumerate() {
                let c = c as char;
                let val = parse_value(&mut cursor, c)?;

                if definition.timestamp_field_idx == Some(idx) {
                    match &val {
                        LogValue::UInt(v) => current_ts = *v * 1000,
                        LogValue::Int(v) => current_ts = (*v as u64) * 1000,
                        _ => {}
                    }
                }

                fields.push((idx, val));
            }

            if current_ts > 0 {
                self.last_timestamp = current_ts;
            } else {
                current_ts = self.last_timestamp;
            }

            return Ok(ArduFrame::ArduMessage(ArduMessage {
                type_id: header.msg_id,
                current_ts,
                fields,
            }));
        }

        Err(anyhow!(
            "Error: Unknown msg ID {} at position {}.",
            header.msg_id,
            file.stream_position()?
        ))
    }
}

pub enum ArduFrame {
    ArduDefinition(ArduDefinition),
    ArduMessage(ArduMessage),
    Eof,
}
