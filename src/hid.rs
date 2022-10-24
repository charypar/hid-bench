use std::time::Duration;

use rusb::{
    self, constants::LIBUSB_REQUEST_GET_DESCRIPTOR, DeviceHandle, InterfaceDescriptor, UsbContext,
};

mod basic;
mod parser;

use basic::BasicItems;

pub use self::parser::ReportParser;

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
    pub fn decode(&self) -> ReportParser {
        ReportParser::new(self.basic_items())
    }

    pub fn basic_items(&self) -> BasicItems {
        BasicItems::new(&self.bytes)
    }
}
