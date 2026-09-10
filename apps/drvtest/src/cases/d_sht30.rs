//! d_sht30：SHT30 温湿度虚拟器件（I2C 0x44，Sensirion）全链路。
//! 判据（与模拟器 vperiph/i2c/sht30.rs 同一协议）：
//!   触发测量（0xE000）→ 读 6 字节：T_hi T_lo CRC RH_hi RH_lo CRC
//!   温度 raw=0x6666（25.0°C）、湿度 raw=0x8000（50.0%）

use rtos_app_sdk::device::I2cXfer;
use rtos_app_sdk::info;
use rtos_app_sdk::ioctl::{I2C_IOCTL_MASTER_READ, I2C_IOCTL_MASTER_WRITE};

use crate::cap;
use crate::verdict::{Ctx, Verdict};

const SHT30_ADDR: u16 = 0x44;

pub fn temp_humi(_ctx: &mut Ctx) -> Verdict {
    let dev = match cap::open("i2c0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };
    // 触发高重复度测量：START(W) → 0xE0 0x00
    let mut cmd = [0xE0u8, 0x00];
    let mut xw = I2cXfer { addr: SHT30_ADDR, buf: cmd.as_mut_ptr(), len: 2, result: 0 };
    if dev.ioctl(I2C_IOCTL_MASTER_WRITE, (&mut xw as *mut I2cXfer).cast()) != 0 || xw.result != 0 {
        return Verdict::Fail("SHT30 触发测量失败");
    }
    // 读 6 字节测量数据
    let mut buf = [0u8; 6];
    let mut xr = I2cXfer { addr: SHT30_ADDR, buf: buf.as_mut_ptr(), len: 6, result: 0 };
    if dev.ioctl(I2C_IOCTL_MASTER_READ, (&mut xr as *mut I2cXfer).cast()) != 0 || xr.result != 0 {
        return Verdict::Fail("SHT30 读数据失败");
    }
    let t_raw = ((buf[0] as u16) << 8) | buf[1] as u16;
    let rh_raw = ((buf[3] as u16) << 8) | buf[4] as u16;
    // 25.0°C → 0x6666=26214；50.0%RH → 0x8000=32768
    if t_raw != 26214 || rh_raw != 32768 {
        return Verdict::Fail("SHT30 温湿度数值不符");
    }
    let t_c = -45.0 + 175.0 * (t_raw as f32) / 65535.0;
    let rh_pct = 100.0 * (rh_raw as f32) / 65535.0;
    info!("sht30", "T={t_c:.1}°C RH={rh_pct:.1}% 正确");
    Verdict::Pass("SHT30 25.0°C / 50.0%RH 正确")
}
