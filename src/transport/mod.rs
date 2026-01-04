pub mod i2c;

pub trait Transport {
    fn write(&mut self, data: &[u8]) -> std::io::Result<()>;
    fn read(&mut self, len: usize) -> std::io::Result<Vec<u8>>;
}
