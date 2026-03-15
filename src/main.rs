/*
 * SPDX-License-Identifier: Apache-2.0
 *
 * Copyright The Asahi Linux Contributors
 */

pub mod cd321x;
#[cfg(target_os = "linux")]
pub mod sysfs;

#[cfg(target_os = "linux")]
use crate::sysfs::get_i2c_dev_from_connector;
use env_logger::Env;
use log::{error, info};
use std::{fs, process::ExitCode};

#[derive(Debug)]
#[allow(dead_code)]
enum Error {
    Device,
    DeviceNotFound,
    Compatible,
    FeatureMissing,
    TypecController,
    InvalidArgument,
    ReconnectTimeout,
    ControllerTimeout,
    I2C,
    Io(std::io::Error),
    Utf8(std::str::Utf8Error),
    Parse(std::num::ParseIntError),
}

type Result<T> = std::result::Result<T, Error>;

#[cfg(not(target_os = "linux"))]
fn get_i2c_dev_fromconnector(&connector: str) -> Result<(String, u16)> {
    Err(Error::DeviceNotFound)
}

fn vdmtool() -> Result<()> {
    let matches = clap::command!()
        .arg(
            clap::arg!(-b --bus [BUS] "i2c bus of the USB-C controller device.")
                .default_value("/dev/i2c-0"),
        )
        .arg(
            clap::arg!(-a --address [ADDRESS] "i2c target address of the USB-C controller device.")
                .default_value("0x38"),
        )
        .arg(clap::arg!(-c --connector [CONNECTOR] "(Partial) connector label of the USB-C controller device."))
        .subcommand(
            clap::Command::new("reboot")
                .about("reboot the target")
                .subcommand(
                    clap::Command::new("serial").about("reboot the target and enter serial mode"),
                )
                .subcommand(
                    clap::Command::new("debugusb")
                        .about("reboot the target and enter Debug USB mode"),
                ),
        )
        // dummy commands to display help for "reboot serial/debugusb"
        .subcommand(
            clap::Command::new("reboot serial").about("reboot the target and enter serial mode"),
        )
        .subcommand(
            clap::Command::new("reboot debugusb")
                .about("reboot the target and enter Debug USB mode"),
        )
        .subcommand(clap::Command::new("serial").about("enter serial mode on both ends"))
        .subcommand(clap::Command::new("debugusb").about("enter Debug USB mode on target"))
        .subcommand(clap::Command::new("dfu").about("put the target into DFU mode"))
        .subcommand(clap::Command::new("disconnect").about("Simulate USB unplug/plug"))
        .subcommand(clap::Command::new("nop").about("Do nothing"))
        .arg_required_else_help(true)
        .get_matches();

    let compat: Vec<u8> = fs::read("/proc/device-tree/compatible").map_err(Error::Io)?;
    let compat = std::str::from_utf8(&compat[0..10]).map_err(Error::Utf8)?;
    let (manufacturer, device) = compat.split_once(",").ok_or(Error::Compatible)?;
    if manufacturer != "apple" {
        error!("Host is not an Apple silicon system: \"{compat}\"");
        return Err(Error::Compatible);
    }

    let addr: u16;
    let bus: String;

    match matches.get_one::<String>("connector") {
        Some(connector) => {
            let connector = connector.to_ascii_lowercase();
            (bus, addr) = get_i2c_dev_from_connector(&connector)?
        }
        None => {
            let addr_str = matches.get_one::<String>("address").unwrap();
            if let Some(stripped) = addr_str.strip_prefix("0x") {
                addr = u16::from_str_radix(stripped, 16).map_err(Error::Parse)?;
            } else {
                addr = addr_str.parse::<u16>().map_err(Error::Parse)?;
            }
            bus = matches.get_one::<String>("bus").unwrap().to_string();
        }
    }
    info!("Using I2C bus:{bus} address:{addr:#x}");

    let code = device.to_uppercase();
    let mut device = cd321x::Device::new(&bus, addr, code)?;

    match matches.subcommand() {
        Some(("dfu", _)) => {
            device.dfu()?;
        }
        Some(("reboot", args)) => match args.subcommand() {
            Some(("serial", _)) => {
                device.reboot_wait()?;
                device.serial()?;
            }
            Some(("debugusb", _)) => {
                device.reboot_wait()?;
                device.debugusb()?;
            }
            None => {
                device.reboot()?;
            }
            _ => {}
        },
        Some(("nop", _)) => {}
        Some(("serial", _)) => {
            device.serial()?;
        }
        Some(("debugusb", _)) => {
            device.debugusb()?;
        }
        Some(("disconnect", _)) => {
            device.disconnect()?;
        }
        _ => {}
    }
    Ok(())
}

fn main() -> ExitCode {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    match vdmtool() {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            error!("vdmtool: {:?}", e);
            ExitCode::FAILURE
        }
    }
}
