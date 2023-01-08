// 1st level: Parse basic items

use std::fmt::{Debug, Display};

#[derive(Debug)]
pub struct BasicItems<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> BasicItems<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        BasicItems { bytes, offset: 0 }
    }
}

impl<'a> BasicItems<'a> {
    fn item_header(header: u8) -> (usize, u8, u8) {
        let sizes: [usize; 4] = [0, 1, 2, 4];

        let size = sizes[(header & 0b11) as usize];
        let item_type = (header & 0b1100) >> 2;
        let tag = (header & 0b11110000) >> 4;

        (size, item_type, tag)
    }
}

impl<'a> Iterator for BasicItems<'a> {
    type Item = BasicItem;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.bytes.len() {
            return None;
        }

        let (size, item_type, tag) = Self::item_header(self.bytes[self.offset]);

        let mut data = 0u32;
        for byte_idx in 0..size {
            // build up from little-endian ordered bytes
            data |= (self.bytes[self.offset + 1 + byte_idx] as u32) << (byte_idx * 8);
        }

        self.offset += size + 1;

        Some(BasicItem::new(item_type, tag, data, size))
    }
}

// NOTE only short items are supported
#[derive(Debug)]
pub enum BasicItem {
    Main(MainItem),
    Global(GlobalItem),
    Local(LocalItem),
    Reserved,
}

impl BasicItem {
    fn new(item_type: u8, tag: u8, data: u32, size: usize) -> Self {
        match item_type {
            0 => Self::Main(MainItem::new(tag, data)),
            1 => Self::Global(GlobalItem::new(tag, data)),
            2 => Self::Local(LocalItem::new(tag, data, size)),
            _ => Self::Reserved,
        }
    }
}

#[derive(Debug)]
pub enum MainItem {
    Input(InputItemData),
    Output(OutputItemData),
    Feature(FeatureItemData),
    Collection(Collection),
    EndCollection,
    Reserved,
}

impl MainItem {
    fn new(tag: u8, data: u32) -> Self {
        match tag {
            0b1000 => Self::Input(InputItemData { data }),
            0b1001 => Self::Output(OutputItemData { data }),
            0b1011 => Self::Feature(FeatureItemData { data }),
            0b1010 => Self::Collection(Collection::new(data as u8)),
            0b1100 => Self::EndCollection,
            _ => Self::Reserved,
        }
    }
}

#[derive(Clone, Copy)]
pub struct InputItemData {
    pub data: u32,
}

impl InputItemData {
    pub fn data(&self) -> bool {
        self.data & 1 == 0
    }

    pub fn constant(&self) -> bool {
        !self.data()
    }

    pub fn array(&self) -> bool {
        self.data & 2 == 0
    }

    pub fn variable(&self) -> bool {
        !self.array()
    }

    pub fn absolute(&self) -> bool {
        self.data & 2_u32.pow(2) == 0
    }

    pub fn relative(&self) -> bool {
        !self.absolute()
    }

    pub fn no_wrap(&self) -> bool {
        self.data & 2_u32.pow(3) == 0
    }

    pub fn wrap(&self) -> bool {
        !self.no_wrap()
    }

    pub fn linear(&self) -> bool {
        self.data & 2_u32.pow(4) == 0
    }

    pub fn non_linear(&self) -> bool {
        !self.linear()
    }

    pub fn preferred(&self) -> bool {
        self.data & 2_u32.pow(5) == 0
    }

    pub fn no_preferred(&self) -> bool {
        !self.preferred()
    }

    pub fn no_null(&self) -> bool {
        self.data & 2_u32.pow(6) == 0
    }

    pub fn null(&self) -> bool {
        !self.no_null()
    }

    pub fn bit_field(&self) -> bool {
        self.data & 2_u32.pow(8) == 0
    }

    pub fn buffered_bytes(&self) -> bool {
        !self.bit_field()
    }
}

impl Debug for InputItemData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for InputItemData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            vec![
                if self.data() { "Data" } else { "Const" },
                if self.array() { "Array" } else { "Variable" },
                if self.absolute() {
                    "Absolute"
                } else {
                    "Relative"
                },
                if self.no_wrap() { "No Wrap" } else { "Wrap" },
                if self.linear() { "Linear" } else { "Nonlinear" },
                if self.preferred() {
                    "Preferred State"
                } else {
                    "No Preferred State"
                },
                if self.no_null() {
                    "No Null position"
                } else {
                    "Null state"
                },
                if self.bit_field() {
                    "Bit Field"
                } else {
                    "Buffered Bytes"
                },
            ]
            .join(",")
        )
    }
}

#[derive(Debug)]
pub struct OutputItemData {
    pub data: u32,
}

#[derive(Debug)]
pub struct FeatureItemData {
    pub data: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum Collection {
    Physical,
    Application,
    Logical,
    Report,
    NamedArray,
    UsageSwitch,
    UsageModifier,
    Reserved,
    Vendor(u8),
}

impl Collection {
    fn new(data: u8) -> Self {
        match data {
            0 => Self::Physical,
            1 => Self::Application,
            2 => Self::Logical,
            3 => Self::Report,
            4 => Self::NamedArray,
            5 => Self::UsageSwitch,
            6 => Self::UsageModifier,
            n if (7..0x7F).contains(&n) => Self::Reserved,
            n => Self::Vendor(n),
        }
    }
}

#[derive(Debug)]
pub enum GlobalItem {
    UsagePage(u16),
    LogicalMinimum(i32),
    LogicalMaximum(i32),
    PhysicalMinimum(i32),
    PhysicalMaximum(i32),
    UnitExponent(u32),
    Unit(u32), // TODO decode
    ReportSize(u32),
    ReportID(u8),
    ReportCount(u32),
    Push,
    Pop,
    Reserved,
}

impl GlobalItem {
    fn new(tag: u8, data: u32) -> Self {
        match tag {
            0 => Self::UsagePage(data as u16),
            1 => Self::LogicalMinimum(data as i32), // FIXME check this works with signs
            2 => Self::LogicalMaximum(data as i32), // FIXME check this works with signs
            3 => Self::PhysicalMinimum(data as i32), // FIXME check this works with signs
            4 => Self::PhysicalMaximum(data as i32), // FIXME check this works with signs
            5 => Self::UnitExponent(data as u32),
            6 => Self::Unit(data),
            7 => Self::ReportSize(data),
            8 => Self::ReportID(data as u8),
            9 => Self::ReportCount(data),
            10 => Self::Push,
            11 => Self::Pop,
            _ => Self::Reserved,
        }
    }
}

#[derive(Debug)]
pub enum LocalItem {
    Usage(u16),
    UsageMinimum(u16),
    UsageMaximum(u16),
    ExtendedUsage(u16, u16),
    ExtendedUsageMinimum(u16, u16),
    ExtendedUsageMaximum(u16, u16),
    DesignatorIndex(u32),
    DesignatorMinimum(u32),
    DesignatorMaximum(u32),
    StringIndex(u32),
    StringMinimum(u32),
    StringMaximum(u32),
    Delimiter(bool), // true - open, false - close
    Reserved,
}

impl LocalItem {
    fn new(tag: u8, data: u32, size: usize) -> Self {
        match (tag, size) {
            (0, 1) => Self::Usage(data as u16),
            (0, 2) => Self::Usage(data as u16),
            (0, 4) => Self::ExtendedUsage((data >> 16) as u16, (data & 0xFF) as u16),
            (1, 1) => Self::UsageMinimum(data as u16),
            (1, 2) => Self::UsageMinimum(data as u16),
            (1, 4) => Self::ExtendedUsageMinimum((data >> 16) as u16, (data & 0xFF) as u16),
            (2, 1) => Self::UsageMaximum(data as u16),
            (2, 2) => Self::UsageMaximum(data as u16),
            (2, 4) => Self::ExtendedUsageMaximum((data >> 16) as u16, (data & 0xFF) as u16),
            (3, _) => Self::DesignatorIndex(data),
            (4, _) => Self::DesignatorMinimum(data),
            (5, _) => Self::DesignatorMaximum(data),
            (6, _) => Self::StringIndex(data),
            (7, _) => Self::StringMinimum(data),
            (8, _) => Self::StringMaximum(data),
            (9, _) => Self::Delimiter(data != 0),
            (_, _) => Self::Reserved,
        }
    }
}

#[cfg(test)]
mod test {
    use insta::assert_debug_snapshot;

    use super::BasicItems;

    const JOYSTICK: [u8; 101] = [
        0x5, 0x1, 0x9, 0x4, 0xa1, 0x1, 0x9, 0x1, 0xa1, 0x0, 0x5, 0x1, 0x9, 0x30, 0x9, 0x31, 0x15,
        0x0, 0x26, 0xff, 0x3, 0x75, 0xa, 0x95, 0x2, 0x81, 0x2, 0x9, 0x35, 0x15, 0x0, 0x26, 0xff,
        0x0, 0x75, 0x8, 0x95, 0x1, 0x81, 0x2, 0x9, 0x32, 0x9, 0x36, 0x15, 0x0, 0x26, 0xff, 0x0,
        0x75, 0x8, 0x95, 0x2, 0x81, 0x2, 0x5, 0x9, 0x19, 0x1, 0x29, 0xe, 0x15, 0x0, 0x25, 0x1,
        0x75, 0x1, 0x95, 0xe, 0x81, 0x2, 0x5, 0x1, 0x9, 0x39, 0x15, 0x1, 0x25, 0x8, 0x35, 0x0,
        0x46, 0x3b, 0x1, 0x66, 0x14, 0x0, 0x75, 0x4, 0x95, 0x1, 0x81, 0x42, 0x75, 0x2, 0x95, 0x1,
        0x81, 0x1, 0xc0, 0xc0,
    ];

    #[test]
    fn parses_basic_report_descriptor_items() {
        let basic_items = BasicItems::new(&JOYSTICK);
        let parsed = basic_items.collect::<Vec<_>>();

        assert_debug_snapshot!(parsed);
    }
}
