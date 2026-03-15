/*
 * SPDX-License-Identifier: Apache-2.0
 *
 * Copyright The Asahi Linux Contributors
 */

use log::debug;
use std::fs;
use std::path::Path;

use crate::{Error, Result};

pub(crate) fn get_i2c_dev_from_connector(connector: &str) -> Result<(String, u16)> {
    let mut match_len = usize::MAX;
    let mut candidate: Option<(String, u16)> = None;

    // iterate over all i2c devices
    for entry in fs::read_dir("/sys/class/i2c-dev/").map_err(Error::Io)? {
        let path = entry.map_err(Error::Io)?.path();
        if let Some(bus) = path.file_name() {
            // look only into /sys/class/i2c-dev/i2c-[0-9]
            if let Some(bus_id) = bus.to_str().unwrap().strip_prefix("i2c-") {
                let path = path.join("device");
                // find i2c devices with a matching connector label
                if let Some((addr, label_len)) = discover_match(connector, &path, bus_id) {
                    // Use the device with the shortest match so that cases like
                    // "USB-C Back Left" and "USB-C Back Left Middle" can be matched
                    // consistently. "back left" will always match the former.
                    if label_len < match_len {
                        let dev_path = Path::new("/dev").join(bus);
                        match_len = label_len;
                        candidate = Some((dev_path.to_string_lossy().to_string(), addr))
                    }
                }
            }
        }
    }

    candidate.ok_or(Error::DeviceNotFound)
}

fn discover_match(connector: &str, dev_path: &Path, bus_id: &str) -> Option<(u16, usize)> {
    for entry in fs::read_dir(dev_path).ok()? {
        let path = entry.ok()?.path();
        if let Some((bus, addr)) = path.file_name()?.to_str()?.split_once("-") {
            // Only consider I2C devices with the pattern  ("%d-%04x", bus, addr)
            if bus != bus_id {
                continue;
            };
            let label_path = path.join("of_node/connector/label");
            let data = fs::read(label_path).ok()?;
            // convert to lower case for case insensitive match
            let label = std::str::from_utf8(&data).ok()?.to_ascii_lowercase();
            if label.starts_with("usb-c ") && label.contains(connector) {
                let addr = u16::from_str_radix(addr, 16).unwrap();
                debug!("Found connector with label '{label}' for {bus}:{addr:#x}");
                return Some((addr, label.len()));
            }
        }
    }
    None
}
