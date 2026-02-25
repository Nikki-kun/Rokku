//! ArduPilot Dataflash log parser
//!
//! Parses .bin files into structured data arrays grouped by message type.

pub mod pipeline;
pub mod reader;

pub use pipeline::{parse_log, LogRecord, ParsedLog};
pub use reader::{ArduDefinition, ArduFrame, ArduMessage, ArduReader};
