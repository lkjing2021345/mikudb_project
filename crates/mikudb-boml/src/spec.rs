//! BOML 规范定义模块
//!
//! 定义 BOML 二进制格式的魔数、版本号、类型标记和限制常量。

/// BOML 魔数: "BOML" (0x42 0x4F 0x4D 0x4C)
pub const BOML_MAGIC: [u8; 4] = [0x42, 0x4F, 0x4D, 0x4C];

/// BOML 格式版本号
pub const BOML_VERSION: u8 = 1;

/// 单个文档最大大小 (16MB)
pub const MAX_DOCUMENT_SIZE: usize = 16 * 1024 * 1024;

/// 最大嵌套深度 (防止栈溢出)
pub const MAX_NESTING_DEPTH: usize = 100;

/// 最大字符串长度 (16MB)
pub const MAX_STRING_LENGTH: usize = 16 * 1024 * 1024;

/// 最大数组长度 (100 万个元素)
pub const MAX_ARRAY_LENGTH: usize = 1_000_000;

/// BOML 类型标记
///
/// 定义所有 BOML 值类型的标记字节。
/// 特殊优化:
/// - 0x11-0x1B: 常用值的零拷贝标记 (true, false, 0, 空字符串等)
/// - 0x20-0x2F: 小字符串 (0-15 字节, 长度编码在标记中)
/// - 0x30-0x3F: 小整数 (-8 到 7, 值编码在标记中)
/// - 0x40-0x4F: 小数组 (0-15 个元素, 长度编码在标记中)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeMarker {
    /// Null 值
    Null = 0x00,
    /// 布尔值 (后跟 1 字节)
    Boolean = 0x01,
    /// 32 位有符号整数
    Int32 = 0x02,
    /// 64 位有符号整数
    Int64 = 0x03,
    /// 128 位整数
    Int128 = 0x04,
    /// 32 位浮点数
    Float32 = 0x05,
    /// 64 位浮点数
    Float64 = 0x06,
    /// 十进制数
    Decimal = 0x07,
    /// 字符串 (UTF-8)
    String = 0x08,
    /// 二进制数据
    Binary = 0x09,
    /// ObjectId (12 字节)
    ObjectId = 0x0A,
    /// UUID (16 字节)
    Uuid = 0x0B,
    /// 日期时间
    DateTime = 0x0C,
    /// 时间戳
    Timestamp = 0x0D,
    /// 数组
    Array = 0x0E,
    /// 文档(嵌套对象)
    Document = 0x0F,
    /// 正则表达式
    Regex = 0x10,
    /// JavaScript 代码
    JavaScript = 0x1C,
    /// JavaScript 代码（带作用域）
    JavaScriptWithScope = 0x1D,

    // 常用值的特殊标记 (零拷贝优化)
    /// 布尔值 true
    BooleanTrue = 0x11,
    /// 布尔值 false
    BooleanFalse = 0x12,
    /// Int32 值 0
    Int32Zero = 0x13,
    /// Int32 值 1
    Int32One = 0x14,
    /// Int32 值 -1
    Int32NegOne = 0x15,
    /// Int64 值 0
    Int64Zero = 0x16,
    /// Float64 值 0.0
    Float64Zero = 0x17,
    /// 空字符串
    EmptyString = 0x18,
    /// 空数组
    EmptyArray = 0x19,
    /// 空文档
    EmptyDocument = 0x1A,
    /// Null 数组 ([null])
    NullArray = 0x1B,

    // 小值优化 (长度/值编码在标记中)
    /// 小字符串 (0x20-0x2F, 长度 0-15)
    SmallString = 0x20,
    /// 小整数 (0x30-0x3F, 值 -8 到 7)
    SmallInt = 0x30,
    /// 小数组 (0x40-0x4F, 长度 0-15)
    SmallArray = 0x40,
}

impl TypeMarker {
    /// # Brief
    /// 从字节值转换为 TypeMarker
    ///
    /// 支持所有标准类型和小值优化类型。
    ///
    /// # Arguments
    /// * `byte` - 类型标记字节
    ///
    /// # Returns
    /// Some(TypeMarker) 如果是有效标记, 否则 None
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
            0x1C => Some(Self::JavaScript),
            0x1D => Some(Self::JavaScriptWithScope),
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

    /// # Brief
    /// 判断是否为小字符串标记
    ///
    /// # Arguments
    /// * `byte` - 类型标记字节
    ///
    /// # Returns
    /// true 如果在 0x20-0x2F 范围
    pub fn is_small_string(byte: u8) -> bool {
        (0x20..0x30).contains(&byte)
    }

    /// # Brief
    /// 判断是否为小整数标记
    ///
    /// # Arguments
    /// * `byte` - 类型标记字节
    ///
    /// # Returns
    /// true 如果在 0x30-0x3F 范围
    pub fn is_small_int(byte: u8) -> bool {
        (0x30..0x40).contains(&byte)
    }

    /// # Brief
    /// 判断是否为小数组标记
    ///
    /// # Arguments
    /// * `byte` - 类型标记字节
    ///
    /// # Returns
    /// true 如果在 0x40-0x4F 范围
    pub fn is_small_array(byte: u8) -> bool {
        (0x40..0x50).contains(&byte)
    }

    /// # Brief
    /// 从小字符串标记提取长度
    ///
    /// # Arguments
    /// * `byte` - 小字符串标记 (0x20-0x2F)
    ///
    /// # Returns
    /// 字符串长度 (0-15)
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
