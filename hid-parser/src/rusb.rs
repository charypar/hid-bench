use std::time::Duration;

use rusb::{
    self, constants::LIBUSB_REQUEST_GET_DESCRIPTOR, DeviceHandle, InterfaceDescriptor, UsbContext,
};

use crate::{DescriptorType, HidDescriptor, ReportDescriptor};

impl<'a> HidDescriptor<'a> {
    pub fn from_interface_descriptor(interface_descriptor: &'a InterfaceDescriptor) -> Self {
        Self {
            interface_num: interface_descriptor.interface_number(),
            bytes: interface_descriptor.extra(),
        }
    }

    // TODO hide behind rusb flag
    pub fn report_descriptors<'s, T: UsbContext>(
        &'s self,
        device_handle: &'a DeviceHandle<T>,
    ) -> ReportDescriptors<'_, T>
    where
        'a: 's,
    {
        ReportDescriptors {
            index: 0,
            hid_descriptor: self,
            device_handle,
        }
    }
}

pub struct ReportDescriptors<'a, T: UsbContext> {
    index: u8,
    hid_descriptor: &'a HidDescriptor<'a>,
    device_handle: &'a DeviceHandle<T>,
}

// TODO hide behind rusb flag
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
