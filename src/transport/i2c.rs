/*
 * SPDX-License-Identifier: Apache-2.0
 *
 * Copyright The Asahi Linux Contributors
 */

use crate::transport::Transport;
use crate::{Error, Result};
use std::io;

#[cfg(any(target_os = "linux", target_os = "android"))]
use i2cdev::{core::I2CDevice, linux::LinuxI2CDevice};

#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) struct I2cTransport {
    dev: LinuxI2CDevice,
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn open_i2c(bus: &str, addr: u16) -> Result<LinuxI2CDevice> {
    if let Ok(dev) = LinuxI2CDevice::new(bus, addr) {
        return Ok(dev);
    }
    log::info!("Safely opening failed ==> Forcefully opening device...");
    unsafe { LinuxI2CDevice::force_new(bus, addr) }.map_err(|_| Error::I2C)
}

#[cfg(any(target_os = "linux", target_os = "android"))]
impl I2cTransport {
    pub(crate) fn new(bus: &str, addr: u16) -> Result<Self> {
        Ok(Self { dev: open_i2c(bus, addr)? })
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
impl Transport for I2cTransport {
    fn write(&mut self, data: &[u8]) -> io::Result<()> {
        self.dev
            .write(data)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    fn read(&mut self, len: usize) -> io::Result<Vec<u8>> {
        let mut buf = vec![0u8; len];
        self.dev
            .read(&mut buf)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(buf)
    }
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
pub(crate) struct I2cTransport;

#[cfg(not(any(target_os = "linux", target_os = "android")))]
impl I2cTransport {
    pub(crate) fn new(_bus: &str, _addr: u16) -> Result<Self> {
        Err(Error::FeatureMissing)
    }
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
impl Transport for I2cTransport {
    fn write(&mut self, _data: &[u8]) -> io::Result<()> {
        Err(io::Error::new(io::ErrorKind::Unsupported, "i2c transport is linux-only"))
    }

    fn read(&mut self, _len: usize) -> io::Result<Vec<u8>> {
        Err(io::Error::new(io::ErrorKind::Unsupported, "i2c transport is linux-only"))
    }
}
