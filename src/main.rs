use std::time::Instant;

use hidapi;
use rusb;

mod hid;

// Mouse
// const DEVICE: (u16, u16, &str) = (0x2516, 0x0044, "");

// Thrustmaster Joystick
// const DEVICE: (u16, u16, &str) = (0x044F, 0xB10A, "");

// Mad Katz Joystick
// const DEVICE: (u16, u16, &str) = (0x0738, 0x1302, "");

// Spinny
const DEVICE: (u16, u16, &str) = (0x16c0, 0x27dc, "niche.london:Spinny-v0.1");

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

    let mut hid_descriptor: hid::Descriptor;

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
                    hid_descriptor = hid::Descriptor::new(&interface_descriptor);

                    println!(
                        "  - HID ({:04x}h) {} descriptor(s)",
                        hid_descriptor.hid(),
                        hid_descriptor.num_descriptors(),
                    );

                    if let Ok(device) = usb_device.open() {
                        for report_descriptor in hid_descriptor.report_descriptors(device) {
                            println!("    HID Report descriptor:");
                            println!("    - Raw bytes: {:x?} ", report_descriptor,);
                            println!(
                                "    - Basic items: {:?}",
                                report_descriptor.basic_items().collect::<Vec<_>>(),
                            );
                            println!("    - Parser: {:?}", report_descriptor.decode());
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
            Err(err) => {
                println!("Can't read: {:?}", err);
                break;
            }
        }
    }
}
