use crate::{BasicItems, Parser};

#[derive(Debug)]
pub struct ReportDescriptor {
    pub bytes: Vec<u8>,
}

impl ReportDescriptor {
    pub fn decode(&self) -> Parser {
        Parser::new(self.basic_items())
    }

    pub fn basic_items(&self) -> BasicItems {
        BasicItems::new(&self.bytes)
    }
}

#[derive(Debug)]
pub struct HidDescriptor<'a> {
    pub(crate) interface_num: u8, // FIXME this is only used by the rusb integration
    pub(crate) bytes: &'a [u8],
}

#[derive(PartialEq, Eq, Debug)]
pub enum DescriptorType {
    Hid = 0x21,
    Report = 0x22,
    Physical = 0x23,
}

// HID 1.11, section 6.2.1
impl<'a> HidDescriptor<'a> {
    pub fn new(bytes: &'a [u8], interface_num: u8) -> Self {
        Self {
            interface_num,
            bytes,
        }
    }

    pub fn hid(&self) -> u16 {
        ((self.bytes[3] as u16) << 8) | self.bytes[2] as u16
    }

    pub fn num_descriptors(&self) -> u8 {
        self.bytes[5]
    }

    pub(crate) fn descriptor_type(&self, index: usize) -> Option<DescriptorType> {
        if index >= self.num_descriptors() as usize {
            return None;
        }

        match self.bytes[3 * index + 6] {
            0x21 => Some(DescriptorType::Hid),
            0x22 => Some(DescriptorType::Report),
            0x23 => Some(DescriptorType::Physical),
            _ => None,
        }
    }

    pub(crate) fn descriptor_length(&self, index: usize) -> Option<u16> {
        if index >= self.num_descriptors() as usize {
            return None;
        }

        Some(((self.bytes[3 * index + 8] as u16) << 8) | self.bytes[3 * index + 7] as u16)
    }
}
