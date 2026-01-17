/*
 * SPDX-License-Identifier: Apache-2.0
 *
 * Copyright The Asahi Linux Contributors
 */

use crate::{Error, Result};
use i2cdev::{core::I2CDevice, linux::LinuxI2CDevice};
use log::{error, info};
use std::{
    str::FromStr,
    thread,
    time::{Duration, Instant},
};

const RECONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const POLL_WAIT: Duration = Duration::from_millis(100);
const RECONNECT_WAIT: Duration = Duration::from_secs(1);

const TPS_REG_MODE: u8 = 0x03;
const TPS_REG_CMD1: u8 = 0x08;
const TPS_REG_DATA1: u8 = 0x09;
const TPS_REG_POWER_STATUS: u8 = 0x3f;

#[allow(dead_code)]
enum VdmSopType {
    Sop = 0b00,
    SopPrime = 0b01,
    SopPrimePrime = 0b10,
    SopStar = 0b11,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
enum TpsMode {
    TpsModeApp,
    TpsModeBoot,
    TpsModeBist,
    TpsModeDisc,
    TpsModePtch,
    TpsModeDbma,
}

impl FromStr for TpsMode {
    type Err = ();
    fn from_str(input: &str) -> std::result::Result<TpsMode, ()> {
        match input {
            "APP " => Ok(TpsMode::TpsModeApp),
            "BOOT" => Ok(TpsMode::TpsModeBoot),
            "BIST" => Ok(TpsMode::TpsModeBist),
            "DISC" => Ok(TpsMode::TpsModeDisc),
            "PTCH" => Ok(TpsMode::TpsModePtch),
            "DBMa" => Ok(TpsMode::TpsModeDbma),
            _ => Err(()),
        }
    }
}

fn is_invalid_cmd(val: u32) -> bool {
    val == 0x444d4321
}

pub(crate) struct Device {
    i2c: LinuxI2CDevice,
    key: Vec<u8>,
}

/// Try to open the given I2C bus and slave address.
/// Returns a configured LinuxI2CDevice on success.
fn verify_i2c_device(bus: &str, slave_address: u16) -> Result<LinuxI2CDevice> {
    match LinuxI2CDevice::new(bus, slave_address) {
        Ok(dev) => {
            return Ok(dev);
        }
        Err(_) => {} // Fall through to attempt forced open
    }

    info!("Safely opening failed ==> Forcefully opening device...");
    let forced = unsafe { LinuxI2CDevice::force_new(bus, slave_address) };
    match forced {
        Ok(dev) => Ok(dev),
        Err(_) => Err(Error::I2CError),
    }
}

impl Device {
    pub(crate) fn new(bus: &str, address: u16, code: String) -> Result<Self> {
        let mut device = Self {
            i2c: verify_i2c_device(bus, address)?,
            key: code.into_bytes().into_iter().rev().collect::<Vec<u8>>(),
        };
        device.lock(device.key.clone().as_slice())?;
        device.dbma(true)?;

        Ok(device)
    }

    fn exec_cmd(&mut self, cmd_tag: &[u8; 4], in_data: &[u8]) -> Result<()> {
        self.exec_cmd_with_timing(cmd_tag, in_data, Duration::from_secs(1), Duration::ZERO)
    }

    fn exec_cmd_with_timing(
        &mut self,
        cmd_tag: &[u8; 4],
        in_data: &[u8],
        cmd_timeout: Duration,
        res_delay: Duration,
    ) -> Result<()> {
        // First: Check CMD1 Register busy
        {
            let mut status_buf = [0u8; 4];
            self.read_block(TPS_REG_CMD1, &mut status_buf)?;
            let val = u32::from_le_bytes(status_buf);
            if val != 0 && !is_invalid_cmd(val) {
                info!("Busy Check Failed with VAL = {:?}", val);
                return Err(Error::TypecController);
            }
        }

        // Write input Data to DATA1
        if !in_data.is_empty() {
            self.write_block(TPS_REG_DATA1, in_data)?;
            info!("Wrote Data: {:02X?}", in_data);
        }

        // Write 4-byte command tag
        self.write_block(TPS_REG_CMD1, cmd_tag)?;

        // Poll until CMD1 becomes zero or timeout
        let start = Instant::now();
        loop {
            let mut status_buf = [0u8; 4];
            self.read_block(TPS_REG_CMD1, &mut status_buf)?;
            let val = u32::from_le_bytes(status_buf);

            if is_invalid_cmd(val) {
                info!("Invalid Command");
                return Err(Error::InvalidArgument);
            }

            if val == 0 {
                break;
            }
            if start.elapsed() > cmd_timeout {
                return Err(Error::ControllerTimeout);
            }
        }
        thread::sleep(res_delay);

        Ok(())
    }

    fn write_block(&mut self, reg: u8, data: &[u8]) -> Result<()> {
        let mut buf = Vec::with_capacity(1 + 1 + data.len());
        let size: u8 = data.len().try_into().unwrap();
        buf.push(reg);
        buf.push(size);
        buf.extend_from_slice(data);
        self.i2c.write(&buf).map_err(|_| Error::I2CError)?;
        Ok(())
    }

    fn read_block(&mut self, reg: u8, buf: &mut [u8]) -> Result<()> {
        self.i2c.write(&[reg]).map_err(|_| Error::I2CError)?;
        let mut internal_buf = vec![0u8; buf.len() + 1];
        self.i2c
            .read(&mut internal_buf)
            .map_err(|_| Error::I2CError)?;
        buf.copy_from_slice(&internal_buf[1..=buf.len()]);

        Ok(())
    }

    fn get_mode(&mut self) -> Result<TpsMode> {
        let mut buf = [0u8; 4];
        self.read_block(TPS_REG_MODE, &mut buf)?;
        let s = std::str::from_utf8(&buf).unwrap();
        let m = TpsMode::from_str(s).map_err(|_| Error::TypecController)?;

        Ok(m)
    }

    fn lock(&mut self, key: &[u8]) -> Result<()> {
        self.exec_cmd(b"LOCK", &key)
    }

    fn dbma(&mut self, debug: bool) -> Result<()> {
        let data: [u8; 1] = if debug { [1] } else { [0] };
        self.exec_cmd(b"DBMa", &data)?;
        if self.get_mode()? != TpsMode::TpsModeDbma {
            return Err(Error::TypecController);
        }
        Ok(())
    }

    fn vdms(&mut self, sop: VdmSopType, vdos: &[u32]) -> Result<()> {
        if vdos.is_empty() || vdos.len() > 7 {
            return Err(Error::InvalidArgument);
        }
        if self.get_mode()? != TpsMode::TpsModeDbma {
            return Err(Error::TypecController);
        }
        let data = [
            vec![((sop as u8) << 4) | vdos.len() as u8],
            vdos.iter().flat_map(|val| val.to_le_bytes()).collect(),
        ]
        .concat();
        self.exec_cmd_with_timing(
            b"VDMs",
            &data,
            Duration::from_millis(200),
            Duration::from_millis(200),
        )
    }

    fn dven(&mut self, vdos: &[u32]) -> Result<()> {
        let data: Vec<u8> = vdos.iter().flat_map(|val| val.to_le_bytes()).collect();
        self.exec_cmd(b"DEVn", &data)
    }

    fn check_connected(&mut self) -> Result<bool> {
        let mut buf = [0u8; 2];
        self.read_block(TPS_REG_POWER_STATUS, &mut buf)?;
        let power_status = u16::from_le_bytes(buf);
        Ok((power_status & 1) != 0)
    }

    pub(crate) fn dfu(&mut self) -> Result<()> {
        let vdos: [u32; 3] = [0x5ac8012, 0x106, 0x80010000];
        info!("Rebooting target into DFU mode...");
        self.vdms(VdmSopType::SopStar, &vdos)
    }

    pub(crate) fn reboot(&mut self) -> Result<()> {
        let vdos: [u32; 3] = [0x5ac8012, 0x105, 0x80000000];
        info!("Rebooting target into normal mode...");
        self.vdms(VdmSopType::SopStar, &vdos)
    }

    pub(crate) fn reboot_serial(&mut self) -> Result<()> {
        self.reboot()?;
        info!("Waiting for connection...");

        thread::sleep(RECONNECT_WAIT);

        let now = Instant::now();
        loop {
            if self.check_connected().unwrap_or(false) {
                break;
            }
            thread::sleep(POLL_WAIT);
            if now.elapsed() > RECONNECT_TIMEOUT {
                error!("Timeout while waiting ");
                return Err(Error::ReconnectTimeout);
            }
        }
        info!(" Connected");
        thread::sleep(RECONNECT_WAIT);
        self.serial()
    }

    pub(crate) fn serial(&mut self) -> Result<()> {
        let vdos: [u32; 2] = [0x5ac8012, 0x1840306];
        info!("Putting target into serial mode...");
        self.vdms(VdmSopType::SopStar, &vdos)?;
        info!("Putting local end into serial mode... ");
        if self.get_mode()? != TpsMode::TpsModeDbma {
            return Err(Error::TypecController);
        }
        self.dven(&vdos[1..2])
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        let lock: [u8; 4] = [0, 0, 0, 0];
        let _ = self.dbma(false);
        let _ = self.lock(&lock);
    }
}
