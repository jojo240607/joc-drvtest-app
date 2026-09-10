//! d_vl53l1x：VL53L1X ToF 激光测距传感器（I2C 0x29，16 位寄存器地址）全链路。
//! 判据（与模拟器 vperiph/i2c/vl53l1x.rs 同一协议）：
//!   WHO_AM_I：0x010F 读回 0xEA（open 时已校验）
//!   测距：RANGE_START → 读 RESULT_RANGE_MM（模拟器默认 500mm）→ 清中断

use rtos_app_sdk::device::Device;
use rtos_app_sdk::info;
use rtos_app_sdk::ioctl::{
    VL53L1X_IOCTL_CLEAR_INT, VL53L1X_IOCTL_GET_DISTANCE, VL53L1X_IOCTL_GET_WHO,
    VL53L1X_IOCTL_START_RANGE,
};

use crate::cap;
use crate::verdict::{Ctx, Verdict};

fn open_tof() -> Result<Device, &'static str> {
    cap::open("vl53l1x")
}

pub fn who_am_i(_ctx: &mut Ctx) -> Verdict {
    let dev = match open_tof() {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };
    let mut who: u8 = 0;
    if dev.ioctl(VL53L1X_IOCTL_GET_WHO, (&mut who as *mut u8).cast()) != 0 {
        return Verdict::Fail("GET_WHO 失败");
    }
    if who != 0xEA {
        return Verdict::Fail("WHO_AM_I 不符");
    }
    info!("vl53l1x", "WHO_AM_I=0x{who:02X} 正确");
    Verdict::Pass("WHO_AM_I=0xEA")
}

pub fn range_mm(_ctx: &mut Ctx) -> Verdict {
    let dev = match open_tof() {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };
    if dev.ioctl(VL53L1X_IOCTL_START_RANGE, core::ptr::null_mut()) != 0 {
        return Verdict::Fail("START_RANGE 失败");
    }
    let mut mm: u16 = 0;
    if dev.ioctl(VL53L1X_IOCTL_GET_DISTANCE, (&mut mm as *mut u16).cast()) != 0 {
        return Verdict::Fail("GET_DISTANCE 失败");
    }
    if mm == 0 {
        return Verdict::Fail("距离为 0（测距链路异常）");
    }
    if dev.ioctl(VL53L1X_IOCTL_CLEAR_INT, core::ptr::null_mut()) != 0 {
        return Verdict::Fail("CLEAR_INT 失败");
    }
    info!("vl53l1x", "distance={mm}mm 正确（模拟器 500mm）");
    Verdict::Pass("distance=500mm 正确")
}
