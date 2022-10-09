use std::time::Instant;

use hidapi;
use rusb;

// Mouse
// const DEVICE: (u16, u16, &str) = (0x2516, 0x0044, "");

// Spinny
const DEVICE: (u16, u16, &str) = (0x16c0, 0x27dc, "niche.london:Spinny-v0.1");

// Joystick
// const DEVICE: (u16, u16, &str) = (0x044F, 0xB10A, "");

fn main() {
    let api = hidapi::HidApi::new().expect("Cannot start hidapi");
    let (vid, pid, _) = DEVICE;

    let device_info = api
        .device_list()
        .filter(|info| info.vendor_id() == vid && info.product_id() == pid)
        .next()
        .expect("Could not find device");

    println!(
        "Device {:04X}:{:04X} - Usage Page: {:04X}h, Usage: {:04X}h, Interface: {},  serial: '{:?}', Manufacturer: {},",
        device_info.vendor_id(),
        device_info.product_id(),
        device_info.usage_page(),
        device_info.usage(),
        device_info.interface_number(),
        device_info.serial_number(),
        device_info.manufacturer_string().unwrap_or_default(),
    );

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

    println!("- Device descriptor: {:?}", usb_device_descriptor);
    for cidx in 0..usb_device_descriptor.num_configurations() {
        println!(
            "  - Config descriptor {}: {:?}",
            cidx,
            usb_device
                .config_descriptor(cidx)
                .expect("could not read config descriptor")
        );
    }

    // Open the device

    let hid_device = match api.open(vid, pid) {
        Err(e) => {
            println!("Error opening: {:?}", e);
            return;
        }
        Ok(d) => d,
    };

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

    //    let mut hid_devices: HashSet<(u16, u16)> = HashSet::new();

    // for device in devices.iter() {
    //     let device_desc = device
    //         .device_descriptor()
    //         .expect("cannot read device descriptor");

    //     for cidx in 0..device_desc.num_configurations() {
    //         let config_desc = device
    //             .config_descriptor(cidx)
    //             .expect("Could not read config descriptor");

    //         for interface in config_desc.interfaces() {
    //             for in_desc in interface.descriptors() {
    //                 if in_desc.class_code() == 3 {
    //                     // a HID interface, push device into list
    //                     hid_devices.insert((device_desc.vendor_id(), device_desc.product_id()));
    //                 }
    //             }
    //         }
    //     }
    // }

    // println!(
    //     "Found {} HID devices (out of {} total).",
    //     hid_devices.len(),
    //     devices.len()
    // );

    // for device in devices.iter() {
    //     let device_desc = device
    //         .device_descriptor()
    //         .expect("cannot read device descriptor");
    //     let vidpid = (device_desc.vendor_id(), device_desc.product_id());

    //     if !hid_devices.contains(&vidpid) {
    //         continue;
    //     }

    //     // Pick specific device
    //     if vidpid != VIDPID {
    //         continue;
    //     }

    //     println!(
    //         "\n### Bus {} Device {}, ID {:04X}:{:04X}. Class-Sublass-Protocol: {:02X}h-{:02X}h-{:02X}h",
    //         device.bus_number(),
    //         device.address(),
    //         device_desc.vendor_id(),
    //         device_desc.product_id(),
    //         device_desc.class_code(),
    //         device_desc.sub_class_code(),
    //         device_desc.protocol_code()
    //     );

    //     let mut handle = device.open().expect("Could not open device");

    //     let man_string = match device_desc.manufacturer_string_index() {
    //         Some(idx) => read_string(&handle, idx),
    //         None => "N/A".to_string(),
    //     };

    //     let dev_string = match device_desc.product_string_index() {
    //         Some(idx) => read_string(&handle, idx),
    //         None => "N/A".to_string(),
    //     };

    //     let sn_string = match device_desc.serial_number_string_index() {
    //         Some(idx) => read_string(&handle, idx),
    //         None => "N/A".to_string(),
    //     };

    //     println!(
    //         "Manufacturer: {}\nDevice: {},\nS/N: {}",
    //         man_string, dev_string, sn_string
    //     );

    //     println!("{} configuration(s):", device_desc.num_configurations());

    //     let mut endpoint_address: Option<(u8, u8)> = None;

    //     for cidx in 0..device_desc.num_configurations() {
    //         let config_desc = device
    //             .config_descriptor(cidx)
    //             .expect("Could not read config descriptor");

    //         for interface in config_desc.interfaces() {
    //             for in_desc in interface.descriptors() {
    //                 println!(
    //                     "  Configuration {}, Interface {}, Setting {}, Class-Subclass-Protocol: {:02X}h-{:02X}h-{:02X}h.\n    Extra: {:x?}",
    //                     cidx,
    //                     in_desc.interface_number(),
    //                     in_desc.setting_number(),
    //                     in_desc.class_code(),
    //                     in_desc.sub_class_code(),
    //                     in_desc.protocol_code(),
    //                     in_desc.extra()
    //                 );

    //                 let c_sc_p = (
    //                     in_desc.class_code(),
    //                     in_desc.sub_class_code(),
    //                     in_desc.protocol_code(),
    //                 );

    //                 for en_desc in in_desc.endpoint_descriptors() {
    //                     println!(
    //                         "    Endpoint {}, Address: {}, Max Size: {}, Direction: {:?}, Interval: {} ms, Transfer type: {:?}.\n      Extra: {:x?}",
    //                         en_desc.number(),
    //                         en_desc.address(),
    //                         en_desc.max_packet_size(),
    //                         en_desc.direction(),
    //                         en_desc.interval(),
    //                         en_desc.transfer_type(),
    //                         en_desc.extra(),
    //                     );

    //                     if c_sc_p == (3, 0, 0) && en_desc.direction() == Direction::In {
    //                         endpoint_address =
    //                             Some((in_desc.interface_number(), en_desc.address()));
    //                     }
    //                 }
    //             }
    //         }
    //     }

    //     if let Some((iface, address)) = endpoint_address {
    //         println!(
    //             "\n\nReading interrupt interface {}, endpoint {}",
    //             iface, address
    //         );
    //         let mut buf = [0u8; 64];

    //         match handle.kernel_driver_active(iface) {
    //             Ok(true) => handle
    //                 .detach_kernel_driver(iface)
    //                 .expect("Cannot detach driver"),
    //             _ => (),
    //         };

    //         if let Err(err) = handle.claim_interface(iface) {
    //             println!("Error claiming interface: {:?}", err);
    //         }

    //         loop {
    //             match handle.read_interrupt(address, &mut buf, Duration::from_millis(50)) {
    //                 Ok(n) => println!("{:x?}", &buf[0..n]),
    //                 Err(err) => {
    //                     println!("Error: {:?}", err);
    //                     return;
    //                 }
    //             };
    //         }
    //     }
    // }
}

// fn read_string<T: UsbContext>(dev: &DeviceHandle<T>, index: u8) -> String {
//     let langs = dev
//         .read_languages(Duration::from_millis(500))
//         .expect("Could not read languages");

//     return dev
//         .read_string_descriptor(langs[0], index, Duration::from_millis(500))
//         .expect("Cannot read string");
// }
