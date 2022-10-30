// 1st level: Parse basic items

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
    Collection(Collection), // TODO work out collection type representation
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

#[derive(Debug)]
pub struct InputItemData {
    pub data: u32,
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
