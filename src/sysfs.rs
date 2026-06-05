/*
 * SPDX-License-Identifier: Apache-2.0
 *
 * Copyright The Asahi Linux Contributors
 */

use log::debug;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{Error, Result};

pub(crate) fn get_i2c_dev_from_typec_port(typec_path: &Path) -> Option<(String, u16)> {
    let path = std::fs::canonicalize(typec_path.join("device")).ok()?;

    // First, check that this device is located on an i2c bus
    let bus_id = path
        .parent()?
        .file_name()?
        .to_str()
        .unwrap()
        .strip_prefix("i2c-")?;

    // Only consider I2C devices with the pattern  ("%d-%04x", bus, addr)
    let (bus, addr) = path.file_name()?.to_str()?.split_once("-")?;

    if bus != bus_id {
        return None;
    };

    let addr = u16::from_str_radix(addr, 16).unwrap();
    Some((format!("/dev/i2c-{}", bus), addr))
}

pub(crate) fn get_typec_port_from_connector(connector: &str) -> Result<PathBuf> {
    let mut match_len = usize::MAX;
    let mut candidate: Option<PathBuf> = None;

    // iterate over all typec ports
    for entry in fs::read_dir("/sys/class/typec/").map_err(Error::Io)? {
        let path = entry.map_err(Error::Io)?.path();
        if let Some(port) = path.file_name() {
            // look only into /sys/class/typec/port[0-9]
            if let Some(port_id) = port.to_str().unwrap().strip_prefix("port") {
                if !port_id.chars().all(char::is_numeric) {
                    continue;
                }
                let label_path = path.join("device/of_node/connector/label");
                let Ok(label) = fs::read_to_string(label_path) else {
                    continue;
                };

                // convert to lower case for case insensitive match
                let label = label.to_ascii_lowercase();

                if label.starts_with("usb-c ") && label.contains(connector) {
                    debug!("Found connector with label '{label}' at {path:?}");
                    // Use the device with the shortest match so that cases like
                    // "USB-C Back Left" and "USB-C Back Left Middle" can be matched
                    // consistently. "back left" will always match the former.
                    if label.len() < match_len {
                        candidate = Some(path);
                        match_len = label.len();
                    }
                }
            }
        }
    }

    candidate.ok_or(Error::DeviceNotFound)
}
