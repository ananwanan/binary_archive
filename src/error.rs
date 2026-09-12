use std::fmt;
use std::io;

#[derive(Debug)]
pub enum ArchiveError {
    Io(io::Error),

    InvalidData(String),

    InvalidMagic {
        expected: String,
        actual: String,
    },

    LengthOverflow {
        length: u64,
    },

    /// The encoded value is larger than the configured safety limit.
    LimitExceeded {
        kind: &'static str,
        value: u64,
        limit: u64,
    },

    /// A chunk payload was not fully consumed by its decoder.
    ChunkNotFullyConsumed {
        remaining: u64,
    },
}

pub type ArchiveResult<T> = Result<T, ArchiveError>;

impl From<io::Error> for ArchiveError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => {
                write!(f, "I/O error: {err}")
            }

            Self::InvalidData(message) => {
                write!(f, "invalid data: {message}")
            }

            Self::InvalidMagic { expected, actual } => {
                write!(f, "invalid magic: expected {expected:?}, actual {actual:?}")
            }

            Self::LengthOverflow { length } => {
                write!(f, "length is too large for this platform: {length}")
            }

            Self::LimitExceeded { kind, value, limit } => {
                write!(f, "{kind} is too large: {value} (limit: {limit})")
            }

            Self::ChunkNotFullyConsumed { remaining } => {
                write!(f, "chunk decoder left {remaining} bytes unread")
            }
        }
    }
}

impl std::error::Error for ArchiveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}
