use crate::cd321x::BusDevice;
use crate::{Error, Result};

use std::io::Read;
use std::path::PathBuf;

pub(crate) struct SpmiBusDevice {
    dbgfs_path: PathBuf,
    sid: u8,
}

#[derive(Debug)]
#[repr(u8)]
enum SpmiOpc {
    ExtWrite = 0x00,
    ExtRead = 0x20,
    // Write = 0x40,
    Read = 0x60,
    ZeroWrite = 0x80,
}

impl SpmiBusDevice {
    pub(crate) fn open(bus: &str, target_address: u16) -> Result<SpmiBusDevice> {
        let dbgfs_path: PathBuf = bus.into();

        for file in ["opc", "saddr", "sid", "data"] {
            if !dbgfs_path.join(file).exists() {
                return Err(Error::Spmi);
            }
        }

        Ok(Self {
            dbgfs_path,
            sid: target_address as u8,
        })
    }

    fn write_op(&mut self, opc: SpmiOpc, saddr: u16, data: &[u8]) -> std::io::Result<()> {
        std::fs::write(self.dbgfs_path.join("sid"), format!("0x{:x}", self.sid))?;
        std::fs::write(self.dbgfs_path.join("opc"), format!("0x{:x}", opc as u8))?;
        std::fs::write(self.dbgfs_path.join("saddr"), format!("0x{:x}", saddr))?;
        std::fs::write(self.dbgfs_path.join("data"), data)?;
        Ok(())
    }

    fn read_op(&mut self, opc: SpmiOpc, saddr: u16, data: &mut [u8]) -> std::io::Result<()> {
        std::fs::write(self.dbgfs_path.join("sid"), format!("0x{:x}", self.sid))?;
        std::fs::write(self.dbgfs_path.join("opc"), format!("0x{:x}", opc as u8))?;
        std::fs::write(self.dbgfs_path.join("saddr"), format!("0x{:x}", saddr))?;
        let mut data_file = std::fs::File::open(self.dbgfs_path.join("data"))?;
        data_file.read_exact(data)
    }

    fn select_reg(&mut self, reg: u8) -> std::io::Result<()> {
        self.write_op(SpmiOpc::ZeroWrite, 0, &[reg])?;
        let mut buf = [0u8; 1];
        while {
            self.read_op(SpmiOpc::Read, 0, &mut buf[..])?;
            buf[0] != reg
        } {
            if buf[0] != reg | 0x80 {
                return Err(std::io::Error::other("failed to select reg"));
            }
        }
        Ok(())
    }
}

impl BusDevice for SpmiBusDevice {
    fn write_block(&mut self, reg: u8, data: &[u8]) -> Result<()> {
        self.select_reg(reg).map_err(|_| Error::Spmi)?;
        self.write_op(SpmiOpc::ExtWrite, 0xa0, data)
            .map_err(|_| Error::Spmi)?;
        Ok(())
    }

    fn read_block(&mut self, reg: u8, buf: &mut [u8]) -> Result<()> {
        self.select_reg(reg).map_err(|_| Error::Spmi)?;
        self.read_op(SpmiOpc::ExtRead, 0x20, buf)
            .map_err(|_| Error::Spmi)?;
        Ok(())
    }
}
