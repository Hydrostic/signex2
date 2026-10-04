//! OVK voice-pack and NWA audio readers.

pub mod nwa;
pub mod ovk;

pub use nwa::{NwaAudio, NwaHeader, decode_nwa, parse_nwa_header};
pub use ovk::{OvkEntry, OvkIndex, parse_ovk};

use crate::DecodeError;

pub type AudioDecodeError = DecodeError;
