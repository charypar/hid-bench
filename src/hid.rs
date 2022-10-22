use std::time::Duration;

use rusb::{
    self, constants::LIBUSB_REQUEST_GET_DESCRIPTOR, DeviceHandle, InterfaceDescriptor, UsbContext,
};

#[derive(PartialEq, Debug)]
pub enum DescriptorType {
    HID = 0x21,
    Report = 0x22,
    Physical = 0x23,
}

#[derive(Debug)]
pub struct Descriptor<'a> {
    interface_num: u8,
    bytes: &'a [u8],
}

// HID 1.11, section 6.2.1
impl<'a> Descriptor<'a> {
    pub fn new(interface_descriptor: &'a InterfaceDescriptor) -> Self {
        Self {
            interface_num: interface_descriptor.interface_number(),
            bytes: interface_descriptor.extra(),
        }
    }

    pub fn hid(&self) -> u16 {
        ((self.bytes[3] as u16) << 8) | self.bytes[2] as u16
    }

    pub fn num_descriptors(&self) -> u8 {
        self.bytes[5]
    }

    pub fn report_descriptors<T: UsbContext>(
        &self,
        device_handle: DeviceHandle<T>,
    ) -> ReportDescriptors<'_, T> {
        ReportDescriptors {
            index: 0,
            hid_descriptor: self,
            device_handle,
        }
    }

    fn descriptor_type(&self, index: usize) -> Option<DescriptorType> {
        if index >= self.num_descriptors() as usize {
            return None;
        }

        match self.bytes[3 * index + 6] {
            0x21 => Some(DescriptorType::HID),
            0x22 => Some(DescriptorType::Report),
            0x23 => Some(DescriptorType::Physical),
            _ => None,
        }
    }

    fn descriptor_length(&self, index: usize) -> Option<u16> {
        if index >= self.num_descriptors() as usize {
            return None;
        }

        Some(((self.bytes[3 * index + 8] as u16) << 8) | self.bytes[3 * index + 7] as u16)
    }
}

pub struct ReportDescriptors<'a, T: UsbContext> {
    index: u8,
    hid_descriptor: &'a Descriptor<'a>,
    device_handle: DeviceHandle<T>,
}

impl<'a, T: UsbContext> Iterator for ReportDescriptors<'a, T> {
    type Item = ReportDescriptor;

    fn next(&mut self) -> Option<Self::Item> {
        // find next Report descriptor
        loop {
            let descriptor_type = match self.hid_descriptor.descriptor_type(self.index as usize) {
                Some(t) => t,
                None => {
                    return None;
                }
            };

            if descriptor_type == DescriptorType::Report {
                break;
            }

            self.index += 1;
        }

        // Constrcut the Get_Descriptor request

        let descriptor_length = self
            .hid_descriptor
            .descriptor_length(self.index as usize)
            .expect("Index no longer valid");
        let descriptor_type = self
            .hid_descriptor
            .descriptor_type(self.index as usize)
            .expect("Index no longer valid");

        let request_type = rusb::request_type(
            rusb::Direction::In,
            rusb::RequestType::Standard,
            rusb::Recipient::Interface,
        );
        let request: u8 = LIBUSB_REQUEST_GET_DESCRIPTOR;

        let value: u16 = (descriptor_type as u16) << 8 | (self.index as u16);

        let mut bytes: Vec<u8> = (0..descriptor_length).map(|_| 0u8).collect();

        // Perform the request

        let result = self.device_handle.read_control(
            request_type,
            request,
            value,
            self.hid_descriptor.interface_num as u16,
            &mut bytes,
            Duration::from_millis(500),
        );

        self.index += 1;

        match result {
            Ok(len) => Some(ReportDescriptor {
                bytes: Vec::from(&bytes[0..len]),
            }),
            Err(err) => {
                println!("Could not read Report descriptor {:?}", err);
                None
            }
        }
    }
}

#[derive(Debug)]
pub struct ReportDescriptor {
    bytes: Vec<u8>,
}

impl ReportDescriptor {
    pub fn items(&self) -> ReportItems {
        ReportItems {
            basic_items: self.basic_items(),
        }
    }

    pub fn basic_items(&self) -> BasicItems {
        BasicItems {
            bytes: &self.bytes,
            offset: 0,
        }
    }
}

// 1st level: Parse basic items

#[derive(Debug)]
pub struct BasicItems<'a> {
    bytes: &'a [u8],
    offset: usize,
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
            data = data | ((self.bytes[self.offset + 1 + byte_idx] as u32) << (byte_idx * 8));
        }

        self.offset += size + 1;

        println!(
            "Basic item type: {}, tag: {}, size: {}, data: {:04x}",
            item_type, tag, size, data
        );

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
    Collection(u8), // TODO work out collection type representation
    EndCollection,
    Reserved,
}

impl MainItem {
    fn new(tag: u8, data: u32) -> Self {
        match tag {
            0b1000 => Self::Input(InputItemData { data }),
            0b1001 => Self::Output(OutputItemData { data }),
            0b1011 => Self::Feature(FeatureItemData { data }),
            0b1010 => Self::Collection(data as u8),
            0b1100 => Self::EndCollection,
            _ => Self::Reserved,
        }
    }
}

#[derive(Debug)]
pub struct InputItemData {
    data: u32,
}

#[derive(Debug)]
pub struct OutputItemData {
    data: u32,
}

#[derive(Debug)]
pub struct FeatureItemData {
    data: u32,
}

#[derive(Debug)]
pub enum GlobalItem {
    UsagePage(u16),
    LogicalMinimum(i32),
    LogicalMaximum(i32),
    PhysicalMinimum(i32),
    PhysicalMaximum(i32),
    UnitExponent(i32),
    Unit(u32), // TODO decode
    ReportSize(u32),
    ReportID(u32),
    ReportCount(u32),
    Push,
    Pop,
    Reserved,
}

impl GlobalItem {
    fn new(tag: u8, data: u32) -> Self {
        match tag {
            0 => Self::UsagePage(data as u16),
            1 => Self::LogicalMinimum(data as i32),
            2 => Self::LogicalMaximum(data as i32),
            3 => Self::PhysicalMinimum(data as i32),
            4 => Self::PhysicalMaximum(data as i32),
            5 => Self::UnitExponent(data as i32),
            6 => Self::Unit(data),
            7 => Self::ReportSize(data),
            8 => Self::ReportID(data),
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
    DesigntorIndex(u32),
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
            (3, _) => Self::DesigntorIndex(data),
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

// 2nd level, parse into ReportItems

pub struct ReportItems<'a> {
    basic_items: BasicItems<'a>,
}

impl<'a> Iterator for ReportItems<'a> {
    type Item = ReportItem;

    // HID 1.111, Section 5.4
    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

pub struct ReportItem {
    usage: u8,
    usage_page: u8,
    usage_minimum: u8,
    usage_maximum: u8,
    logical_minimum: u8,
    logical_maximum: u8,
    report_size: u8,
    report_count: u8,
}
