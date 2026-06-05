use crate::cd321x::BusDevice;
use crate::{Error, Result};

use i2cdev::{core::I2CDevice, linux::LinuxI2CDevice};
use log::info;

pub(crate) struct I2CBusDevice {
    i2c: LinuxI2CDevice,
}

impl BusDevice for I2CBusDevice {
    fn write_block(&mut self, reg: u8, data: &[u8]) -> Result<()> {
        let mut buf = Vec::with_capacity(1 + 1 + data.len());
        let size: u8 = data.len().try_into().unwrap();
        buf.push(reg);
        buf.push(size);
        buf.extend_from_slice(data);
        self.i2c.write(&buf).map_err(|_| Error::I2C)?;
        Ok(())
    }

    fn read_block(&mut self, reg: u8, buf: &mut [u8]) -> Result<()> {
        self.i2c.write(&[reg]).map_err(|_| Error::I2C)?;
        let mut internal_buf = vec![0u8; buf.len() + 1];
        self.i2c.read(&mut internal_buf).map_err(|_| Error::I2C)?;
        buf.copy_from_slice(&internal_buf[1..=buf.len()]);

        Ok(())
    }
}

impl I2CBusDevice {
    /// Try to open the given I2C bus and target address.
    /// Returns a configured I2CBusDevice on success.
    pub(crate) fn open(bus: &str, target_address: u16) -> Result<I2CBusDevice> {
        if let Ok(dev) = LinuxI2CDevice::new(bus, target_address) {
            return Ok(I2CBusDevice { i2c: dev });
        }
        // Attempt forced open

        info!("Safely opening failed ==> Forcefully opening device...");
        let forced = unsafe { LinuxI2CDevice::force_new(bus, target_address) };
        match forced {
            Ok(dev) => Ok(I2CBusDevice { i2c: dev }),
            Err(_) => Err(Error::I2C),
        }
    }
}
