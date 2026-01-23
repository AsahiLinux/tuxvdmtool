/*
 * SPDX-License-Identifier: Apache-2.0
 *
 * Copyright The Asahi Linux Contributors
 */


#![cfg_attr(not(any(target_os = "linux", target_os = "android")), allow(dead_code, unused_imports))]
#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn main() {
    eprintln!("tuxvdmtool currently supports Linux only.");
}

pub mod cd321x;
pub mod transport;

use env_logger::Env;
use log::error;
use std::{fs, process::ExitCode};
use transport::i2c::I2cTransport;

#[derive(Debug)]
#[allow(dead_code)]
enum Error {
    Device,
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

fn vdmtool() -> Result<()> {
    let matches = clap::command!()
        .arg(
            clap::arg!(-b --bus [BUS] "i2c bus of the USB-C controller device.")
                .default_value("/dev/i2c-0"),
        )
        .arg(
            clap::arg!(-a --address [ADDRESS] "i2c slave address of the USB-C controller device.")
                .default_value("0x38"),
        )
        .subcommand(
            clap::Command::new("reboot")
                .about("reboot the target")
                .subcommand(
                    clap::Command::new("serial").about("reboot the target and enter serial mode"),
                ),
        )
        .subcommand(
            clap::Command::new("reboot serial").about("reboot the target and enter serial mode"),
        )
        .subcommand(clap::Command::new("serial").about("enter serial mode on both ends"))
        .subcommand(clap::Command::new("debugusb").about("enter Debug USB mode on target"))
        .subcommand(clap::Command::new("dfu").about("put the target into DFU mode"))
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

    let addr_str = matches.get_one::<String>("address").unwrap();
    let addr: u16;
    if let Some(stripped) = addr_str.strip_prefix("0x") {
        addr = u16::from_str_radix(stripped, 16).map_err(Error::Parse)?;
    } else {
        addr = addr_str.parse::<u16>().map_err(Error::Parse)?;
    }

    let code = device.to_uppercase();

    let bus = matches.get_one::<String>("bus").unwrap();
    let mut transport = I2cTransport::new(bus.as_str(), addr)?;
    let mut device = cd321x::Device::new(&mut transport, &code)?;

    match matches.subcommand() {
        Some(("dfu", _)) => {
            device.dfu()?;
        }
        Some(("reboot", args)) => match args.subcommand() {
            Some(("serial", _)) => {
                device.reboot_serial()?;
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
        _ => {}
    }
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "android"))]
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
