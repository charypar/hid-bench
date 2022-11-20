use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};
use clap::{Parser as ClapParser, Subcommand, ValueEnum};

mod hid;
use hid::{Collection, HidDescriptor, Input, Parser, ReportDescriptor};
use hidapi::HidApi;
use rusb::{Device, GlobalContext};

#[derive(Debug, ClapParser)]
#[command(name = "hid-bencch")]
#[command(about = "USB HID test bencch", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Lists USB HID devices
    List,
    /// Shows a report descriptor of a given device
    Report {
        #[arg(value_name = "VID:PID", long, short)]
        device: String,
        #[arg(value_enum, long, short)]
        format: Option<ReportFormat>,
    },
    /// Logs input reports from the device
    Log {
        #[arg(value_name = "VID:PID", long, short)]
        device: String,
        #[arg(value_name = "INTERFACE_NUMBER", long, short)]
        interface: String,
        #[arg(value_enum, long, short)]
        format: Option<LogFormat>,
    },
}

#[derive(ValueEnum, Debug, Clone, PartialEq, Eq)]
enum ReportFormat {
    Raw,
    Items,
    Parsed,
}

#[derive(ValueEnum, Debug, Clone, PartialEq, Eq)]
enum LogFormat {
    Raw,
    Compact,
    Full,
}

fn main() -> Result<()> {
    let args = Cli::parse();
    let cmd = args.command;

    if let Commands::List = cmd {
        return cmd_list();
    }

    let hid_devices = hid_devices()?;

    if let Commands::Report { device, format } = cmd {
        let format = format.unwrap_or(ReportFormat::Items);
        let (vid, pid) = parse_vid_pid(&device)?;

        let usb_device = find_device(&hid_devices, vid, pid)
            .ok_or_else(|| anyhow!("Could not find a HID device with vid {vid} pid {pid}"))?;
        let report_descriptors = get_report_descriptors(usb_device)?;

        return cmd_report(&report_descriptors, format);
    }

    if let Commands::Log {
        device,
        interface,
        format,
    } = cmd
    {
        let format = format.unwrap_or(LogFormat::Compact);
        let (vid, pid) = parse_vid_pid(&device)?;
        let interface: u8 =
            str::parse(&interface).map_err(|_| anyhow!("Interface must be a number"))?;

        let usb_device = find_device(&hid_devices, vid, pid)
            .ok_or_else(|| anyhow!("Could not find a HID device with vid {vid} pid {pid}"))?;
        let report_descriptors = get_report_descriptors(usb_device)?;
        let parser = report_descriptors
            .get(&interface)
            .ok_or_else(|| anyhow!("Cannot find interface #{}", interface))?
            .first()
            .ok_or_else(|| anyhow!("No report descriptors for interface #{}", interface))?
            .decode();

        cmd_log(vid, pid, &parser, format)?;
    }

    Ok(())
}

fn cmd_list() -> Result<()> {
    // FIXME do this with rusb instead
    for device in hid_devices()?.iter() {
        let descriptor = device.device_descriptor()?;

        let handle = device.open()?;

        let languages = handle.read_languages(Duration::from_millis(100))?;

        if languages.is_empty() {
            println!(
                "[{:04X}:{:04X}]: <device does not support text descriptions>",
                descriptor.vendor_id(),
                descriptor.product_id(),
            );
            continue;
        }

        let language = languages
            .first()
            .expect("languages should not be empty at this point");

        let vendor_string =
            handle.read_manufacturer_string(*language, &descriptor, Duration::from_millis(100))?;
        let product_string =
            handle.read_product_string(*language, &descriptor, Duration::from_millis(100))?;

        println!(
            "[{:04X}:{:04X}]: \"{}: {}\"",
            descriptor.vendor_id(),
            descriptor.product_id(),
            vendor_string,
            product_string,
        );
    }

    Ok(())
}

fn cmd_report(descriptors: &HashMap<u8, Vec<ReportDescriptor>>, fmt: ReportFormat) -> Result<()> {
    for (interface_number, report_descriptors) in descriptors {
        println!("Interface #{}", interface_number);

        for descriptor in report_descriptors {
            // TODO better formats
            match fmt {
                ReportFormat::Raw => println!("{:?}", descriptor.bytes),
                ReportFormat::Items => {
                    println!("{:?}", descriptor.basic_items().collect::<Vec<_>>())
                }
                ReportFormat::Parsed => println!("{:?}", descriptor.decode()),
            }
        }
    }

    Ok(())
}

fn cmd_log(vid: u16, pid: u16, parser: &Parser, fmt: LogFormat) -> Result<()> {
    let api = HidApi::new()?;
    let hid_device = api.open(vid, pid)?;

    let mut buf = [0u8; 64];
    let mut last = Instant::now();

    loop {
        let n = hid_device.read(&mut buf)?;

        let elapsed = last.elapsed().as_millis();
        let bytes = &buf[0..n];

        // TODO better formats
        match fmt {
            LogFormat::Raw => {
                println!("[+{:06} ms]: {:02x?} ", elapsed, bytes);
            }
            LogFormat::Compact => {
                println!(
                    "[+{:06} ms]: {:02x?} = {}",
                    elapsed,
                    bytes,
                    print_report(&parser.parse_input(&buf[0..n]))
                );
            }
            LogFormat::Full => {
                println!(
                    "[+{:06} ms]: {:02x?} = {:?}",
                    elapsed,
                    bytes,
                    &parser.parse_input(&buf[0..n])
                );
            }
        }

        last = Instant::now();
    }
}

fn parse_vid_pid(vidpid: &str) -> Result<(u16, u16)> {
    let parts: Vec<u16> = vidpid
        .split(':')
        .map(|part| {
            u16::from_str_radix(part, 16).map_err(|_| {
                anyhow!("Device must be two 4-digit hexadecimal numbers separated by ':', e.g.  ")
            })
        })
        .collect::<Result<_>>()?;

    Ok((parts[0], parts[1]))
}

fn find_device(
    devices: &[Device<GlobalContext>],
    vid: u16,
    pid: u16,
) -> Option<&Device<GlobalContext>> {
    devices.iter().find(|d| match d.device_descriptor() {
        Ok(desc) => desc.vendor_id() == vid && desc.product_id() == pid,
        _ => false,
    })
}

fn print_report(collection: &Collection<Vec<Input>>) -> String {
    format!(
        "[{}]",
        collection
            .items
            .iter()
            .filter_map(|item| match item {
                hid::CollectionItem::Collection(c) => Some(print_report(c)),
                hid::CollectionItem::Item(inputs) => {
                    if inputs.is_empty() {
                        return None;
                    }

                    Some(
                        inputs
                            .iter()
                            .map(|i| match i.value {
                                hid::InputValue::Bool(v) => format!("{}", v),
                                hid::InputValue::UInt(v) => format!("{}", v),
                                hid::InputValue::Int(v) => format!("{}", v),
                                hid::InputValue::None => "None".to_string(),
                            })
                            .collect::<Vec<_>>()
                            .join(","),
                    )
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn hid_devices() -> Result<Vec<Device<GlobalContext>>> {
    let mut devices = vec![];

    for device in rusb::devices()?.iter() {
        if !is_hid_device(&device)? {
            continue;
        }

        devices.push(device);
    }

    Ok(devices)
}

fn is_hid_device(usb_device: &Device<GlobalContext>) -> Result<bool> {
    let usb_device_descriptor = usb_device.device_descriptor()?;

    for cidx in 0..usb_device_descriptor.num_configurations() {
        let config_descriptor = usb_device.config_descriptor(cidx)?;

        for interface in config_descriptor.interfaces() {
            for interface_descriptor in interface.descriptors() {
                if interface_descriptor.class_code() == 3 {
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}

fn get_report_descriptors(
    usb_device: &Device<GlobalContext>,
) -> Result<HashMap<u8, Vec<ReportDescriptor>>> {
    let mut descriptors = HashMap::new();

    let usb_device_descriptor = usb_device.device_descriptor()?;
    let device_handle = usb_device.open()?;

    for cidx in 0..usb_device_descriptor.num_configurations() {
        let config_descriptor = usb_device.config_descriptor(cidx)?;

        for interface in config_descriptor.interfaces() {
            for interface_descriptor in interface.descriptors() {
                if interface_descriptor.class_code() == 3 {
                    let interface_num = interface_descriptor.interface_number();
                    let hid_descriptor = HidDescriptor::new(&interface_descriptor);
                    let report_descriptors =
                        hid_descriptor.report_descriptors(&device_handle).collect();

                    descriptors.insert(interface_num, report_descriptors);
                }
            }
        }
    }

    Ok(descriptors)
}
