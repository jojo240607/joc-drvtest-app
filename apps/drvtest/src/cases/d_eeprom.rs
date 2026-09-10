//! d_eeprom：AT24Cxx EEPROM 虚拟器件（I2C 0x50，16 位地址指针）全链路。
//! 判据（与模拟器 vperiph/i2c/at24cxx.rs 同一协议）：
//!   字节写 0x0010=0x5A → 随机读回 [0x5A, 0xFF]（出厂全 0xFF，指针递增）

use rtos_app_sdk::device::{Device, I2cXfer};
use rtos_app_sdk::info;
use rtos_app_sdk::ioctl::{I2C_IOCTL_MASTER_READ, I2C_IOCTL_MASTER_WRITE};

use crate::cap;
use crate::verdict::{Ctx, Verdict};

const EEPROM_ADDR: u16 = 0x50;

fn i2c_write(dev: &Device, data: &mut [u8]) -> bool {
    let mut x = I2cXfer { addr: EEPROM_ADDR, buf: data.as_mut_ptr(), len: data.len() as u16, result: 0 };
    dev.ioctl(I2C_IOCTL_MASTER_WRITE, (&mut x as *mut I2cXfer).cast()) == 0 && x.result == 0
}

fn i2c_read(dev: &Device, buf: &mut [u8]) -> bool {
    let mut x = I2cXfer { addr: EEPROM_ADDR, buf: buf.as_mut_ptr(), len: buf.len() as u16, result: 0 };
    dev.ioctl(I2C_IOCTL_MASTER_READ, (&mut x as *mut I2cXfer).cast()) == 0 && x.result == 0
}

pub fn write_read_roundtrip(_ctx: &mut Ctx) -> Verdict {
    let dev = match cap::open("i2c0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };
    // 字节写：START(W) → 0x00 0x10 0x5A（写地址 0x0010 = 0x5A）
    let mut w = [0x00u8, 0x10, 0x5A];
    if !i2c_write(&dev, &mut w) {
        return Verdict::Fail("EEPROM 写失败");
    }
    // 随机读：START(W) → 0x00 0x10 设指针 → START(R) 读 2 字节
    let mut a = [0x00u8, 0x10];
    if !i2c_write(&dev, &mut a) {
        return Verdict::Fail("EEPROM 设地址指针失败");
    }
    let mut buf = [0u8; 2];
    if !i2c_read(&dev, &mut buf) {
        return Verdict::Fail("EEPROM 读失败");
    }
    if buf[0] != 0x5A || buf[1] != 0xFF {
        return Verdict::Fail("EEPROM 数值不符");
    }
    info!("eeprom", "AT24Cxx 写读回：addr0x0010=0x5A、下字节=0xFF（出厂值）正确");
    Verdict::Pass("EEPROM 字节写/随机读数值一致")
}
