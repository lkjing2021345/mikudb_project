pub const BOML_MAGIC: [u8; 4] = [0x42, 0x4F, 0x4D, 0x4C];
pub const BOML_VERSION: u8 = 1;

pub const MAX_DOCUMENT_SIZE: usize = 16 * 1024 * 1024;
pub const MAX_NESTING_DEPTH: usize = 100;
pub const MAX_STRING_LENGTH: usize = 16 * 1024 * 1024;
pub const MAX_ARRAY_LENGTH: usize = 1_000_000;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeMarker {
    Null = 0x00,
    Boolean = 0x01,
    Int32 = 0x02,
    Int64 = 0x03,
    Int128 = 0x04,
    Float32 = 0x05,
    Float64 = 0x06,
    Decimal = 0x07,
    String = 0x08,
    Binary = 0x09,
    ObjectId = 0x0A,
    Uuid = 0x0B,
    DateTime = 0x0C,
    Timestamp = 0x0D,
    Array = 0x0E,
    Document = 0x0F,
    Regex = 0x10,

    BooleanTrue = 0x11,
    BooleanFalse = 0x12,
    Int32Zero = 0x13,
    Int32One = 0x14,
    Int32NegOne = 0x15,
    Int64Zero = 0x16,
    Float64Zero = 0x17,
    EmptyString = 0x18,
    EmptyArray = 0x19,
    EmptyDocument = 0x1A,
    NullArray = 0x1B,

    SmallString = 0x20,
    SmallInt = 0x30,
    SmallArray = 0x40,
}

impl TypeMarker {
    pub fn from_u8(byte: u8) -> Option<Self> {
        match byte {
            0x00 => Some(Self::Null),
            0x01 => Some(Self::Boolean),
            0x02 => Some(Self::Int32),
            0x03 => Some(Self::Int64),
            0x04 => Some(Self::Int128),
            0x05 => Some(Self::Float32),
            0x06 => Some(Self::Float64),
            0x07 => Some(Self::Decimal),
            0x08 => Some(Self::String),
            0x09 => Some(Self::Binary),
            0x0A => Some(Self::ObjectId),
            0x0B => Some(Self::Uuid),
            0x0C => Some(Self::DateTime),
            0x0D => Some(Self::Timestamp),
            0x0E => Some(Self::Array),
            0x0F => Some(Self::Document),
            0x10 => Some(Self::Regex),
            0x11 => Some(Self::BooleanTrue),
            0x12 => Some(Self::BooleanFalse),
            0x13 => Some(Self::Int32Zero),
            0x14 => Some(Self::Int32One),
            0x15 => Some(Self::Int32NegOne),
            0x16 => Some(Self::Int64Zero),
            0x17 => Some(Self::Float64Zero),
            0x18 => Some(Self::EmptyString),
            0x19 => Some(Self::EmptyArray),
            0x1A => Some(Self::EmptyDocument),
            0x1B => Some(Self::NullArray),
            b if (0x20..0x30).contains(&b) => Some(Self::SmallString),
            b if (0x30..0x40).contains(&b) => Some(Self::SmallInt),
            b if (0x40..0x50).contains(&b) => Some(Self::SmallArray),
            _ => None,
        }
    }

    pub fn is_small_string(byte: u8) -> bool {
        (0x20..0x30).contains(&byte)
    }

    pub fn is_small_int(byte: u8) -> bool {
        (0x30..0x40).contains(&byte)
    }

    pub fn is_small_array(byte: u8) -> bool {
        (0x40..0x50).contains(&byte)
    }

    pub fn small_string_len(byte: u8) -> usize {
        (byte - 0x20) as usize
    }

    pub fn small_int_value(byte: u8) -> i8 {
        (byte - 0x30) as i8
    }

    pub fn small_array_len(byte: u8) -> usize {
        (byte - 0x40) as usize
    }
}
