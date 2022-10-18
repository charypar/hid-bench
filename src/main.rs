use std::time::{Duration, Instant};

use hidapi;
use rusb::{self, constants::LIBUSB_REQUEST_GET_DESCRIPTOR};

// Mouse
// const DEVICE: (u16, u16, &str) = (0x2516, 0x0044, "");

// Thrustmaster Joystick
// const DEVICE: (u16, u16, &str) = (0x044F, 0xB10A, "");

// Mad Katz Joystick
const DEVICE: (u16, u16, &str) = (0x0738, 0x1302, "");

// Spinny
// const DEVICE: (u16, u16, &str) = (0x16c0, 0x27dc, "niche.london:Spinny-v0.1");

fn main() {
    let api = hidapi::HidApi::new().expect("Cannot start hidapi");
    let (vid, pid, _) = DEVICE;

    let device_info = api
        .device_list()
        .filter(|info| info.vendor_id() == vid && info.product_id() == pid)
        .next()
        .expect("Could not find device");

    println!(
        "HID Device {:04X}:{:04X} - Usage Page: {:04X}h, Usage: {:04X}h, Interface: {}",
        device_info.vendor_id(),
        device_info.product_id(),
        device_info.usage_page(),
        device_info.usage(),
        device_info.interface_number(),
    );

    let hid_device = match api.open(vid, pid) {
        Err(e) => {
            println!("Error opening: {:?}", e);
            return;
        }
        Ok(d) => d,
    };

    let mut hid_descriptor: HIDDescriptor;

    // Get the descriptors

    let usb_device = rusb::devices()
        .expect("Could not list USB devices")
        .iter()
        .find(|d| {
            let descriptor = d
                .device_descriptor()
                .expect("could not read device descriptor");

            descriptor.vendor_id() == vid && descriptor.product_id() == pid
        })
        .expect("Could not find device");

    let usb_device_descriptor = usb_device
        .device_descriptor()
        .expect("could not read device descriptor");

    println!(
        "USB Device {:04X}:{:04X}: Class-Subclass-Proto {:02x}h-{:02x}h-{:02x}h - \"{}: {}\"\n  {:?}",
        usb_device_descriptor.vendor_id(),
        usb_device_descriptor.product_id(),
        usb_device_descriptor.class_code(),
        usb_device_descriptor.sub_class_code(),
        usb_device_descriptor.protocol_code(),
        hid_device
            .get_manufacturer_string()
            .unwrap()
            .unwrap_or_default(),
        hid_device.get_product_string().unwrap().unwrap_or_default(),
        // hid_device.get_serial_number_string().unwrap().unwrap_or_default(),
        usb_device_descriptor
    );
    for cidx in 0..usb_device_descriptor.num_configurations() {
        let config_descriptor = usb_device
            .config_descriptor(cidx)
            .expect("could not read config descriptor");

        println!(
            "- Config descriptor extra: {:x?}",
            config_descriptor.extra()
        );

        for interface in config_descriptor.interfaces() {
            let interface_num = interface.number();

            for interface_descriptor in interface.descriptors() {
                if interface_descriptor.class_code() != 3 {
                    continue;
                }

                println!(
                    "  - Interface {} Class-Subclass-Proto: {:02x}h-{:02x}h-{:02x}\n      Extra bytes: {:x?}",
                    interface_num,
                    interface_descriptor.class_code(),
                    interface_descriptor.sub_class_code(),
                    interface_descriptor.protocol_code(),
                    interface_descriptor.extra()
                );

                if interface_descriptor.class_code() == 3 {
                    hid_descriptor = HIDDescriptor::new(interface_descriptor.extra());

                    println!(
                        "  - HID ({:04x}h) {} descriptor(s) type: {:02x}h length: {}",
                        hid_descriptor.hid(),
                        hid_descriptor.num_descriptors(),
                        hid_descriptor.descriptor_type(),
                        hid_descriptor.descriptor_length()
                    );

                    if let Some(device) = rusb::open_device_with_vid_pid(vid, pid) {
                        // FIXME make this read _better_
                        let request_type = rusb::request_type(
                            rusb::Direction::In,
                            rusb::RequestType::Standard,
                            rusb::Recipient::Interface,
                        );
                        let request: u8 = LIBUSB_REQUEST_GET_DESCRIPTOR;

                        let descriptor_type: u8 = 0x22; // Class descriptor, Report
                        let descriptor_index: u8 = 0;

                        let value: u16 = (descriptor_type as u16) << 8 | (descriptor_index as u16);

                        // println!(
                        //     "Control request: request type {:08b}, request: 0x{:02x}, value:  {:02x} + {:08b} = {:016b}",
                        //     request_type, request,
                        //     descriptor_type,
                        //     descriptor_index,
                        //     value
                        // );

                        let mut buf = [0u8; 1024]; // Should be enough...

                        let result = device.read_control(
                            request_type,
                            request,
                            value,
                            interface_num as u16,
                            &mut buf,
                            Duration::from_millis(1500),
                        );

                        match result {
                            Ok(len) => {
                                println!("    HID Report descriptor: {:x?}", &buf[0..len]);
                            }
                            Err(err) => println!("Could not read Report descriptor {:?}", err),
                        }
                    } else {
                        println!("Could not open the device!");
                    }
                }

                for endpoint_descriptor in interface_descriptor.endpoint_descriptors() {
                    println!(
                        "    - Endpoint ({} {:?}) {:?} descriptor: {:?}\n        Extra bytes: {:x?}",
                        endpoint_descriptor.address(),
                        endpoint_descriptor.direction(),
                        endpoint_descriptor.transfer_type(),
                        endpoint_descriptor,
                        endpoint_descriptor.extra()
                    );
                }
            }
        }
    }

    let mut buf = [0u8; 64];
    let mut last = Instant::now();
    println!("\nDevice open, reading...");

    loop {
        match hid_device.read(&mut buf) {
            Ok(n) => {
                let elapsed = last.elapsed().as_millis();
                println!("[+{:06} ms]: {:02x?}", elapsed, &buf[0..n]);
                last = Instant::now();
            }
            Err(err) => println!("Can't read: {:?}", err),
        }
    }
}

struct HIDDescriptor<'a> {
    bytes: &'a [u8],
}

impl<'a> HIDDescriptor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    fn hid(&self) -> u16 {
        ((self.bytes[3] as u16) << 8) | self.bytes[2] as u16
    }

    fn num_descriptors(&self) -> u8 {
        self.bytes[5]
    }

    fn descriptor_type(&self) -> u8 {
        // 0x21 - HID
        // 0x22 - Report
        // 0x23 - Physical
        // 0x24..0x4F - Reserved
        self.bytes[6]
    }

    fn descriptor_length(&self) -> u16 {
        ((self.bytes[8] as u16) << 8) | self.bytes[7] as u16
    }
}
