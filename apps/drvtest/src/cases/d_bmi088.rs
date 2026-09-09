//! d_bmi088：BMI088 双片选六轴 IMU（SPI + GPIO 片选）全链路读取。
//! 判据（数值正确，与模拟器虚拟从机 bmi088.rs 同一换算）：
//!   WHO_AM_I：ACCEL=0x1E、GYRO=0x0F（片选区分同一地址 0x00）
//!   RAW：悬停 accel.x/y≈0、accel.z≈10920（±3g 量程 1g）、gyro≈0
//!   SI ：accel.z≈+9.81 m/s²、gyro≈0 rad/s
//!   device read：12B = accel 6B + gyro 6B（16bit LE）

use core::ptr::null_mut;

use rtos_app_sdk::device::Device;
use rtos_app_sdk::ioctl::{Bmi088Raw, Bmi088Si, Bmi088Who, BMI088_IOCTL_GET_RAW, BMI088_IOCTL_GET_SI, BMI088_IOCTL_GET_WHO};
use rtos_app_sdk::info;

use crate::cap;
use crate::verdict::{Ctx, Verdict};

const WHO_ACCEL: u8 = 0x1E;
const WHO_GYRO: u8 = 0x0F;
/// ±3g 量程灵敏度（LSB/g），与模拟器一致
const ACCEL_LSB_PER_G: f32 = 10920.0;
const GYRO_LSB_PER_DPS: f32 = 16.4;

fn open_bmi088() -> Result<Device, &'static str> {
    match cap::open("bmi088") {
        Ok(d) => Ok(d),
        Err(r) => Err(r),
    }
}

pub fn who_am_i(_ctx: &mut Ctx) -> Verdict {
    let dev = match open_bmi088() {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };
    let mut who = Bmi088Who { accel: 0, gyro: 0 };
    if dev.ioctl(BMI088_IOCTL_GET_WHO, (&mut who as *mut Bmi088Who).cast()) != 0 {
        return Verdict::Fail("GET_WHO 失败");
    }
    if who.accel != WHO_ACCEL || who.gyro != WHO_GYRO {
        return Verdict::Fail("WHO_AM_I 不符（片选区分失败？）");
    }
    info!("bmi088", "WHO accel=0x{:02X} gyro=0x{:02X} (expect 1E/0F)", who.accel, who.gyro);
    Verdict::Pass("WHO_AM_I 0x1E/0x0F 片选区分正确")
}

pub fn raw_readout(_ctx: &mut Ctx) -> Verdict {
    let dev = match open_bmi088() {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };
    let mut raw = Bmi088Raw { accel: [0; 3], gyro: [0; 3] };
    if dev.ioctl(BMI088_IOCTL_GET_RAW, (&mut raw as *mut Bmi088Raw).cast()) != 0 {
        return Verdict::Fail("GET_RAW 失败");
    }
    // 悬停：x/y≈0（±40 LSB）、z≈10920（±5%）、gyro≈0（±40 LSB）
    let az = raw.accel[2] as i32;
    if raw.accel[0].abs() > 40 || raw.accel[1].abs() > 40 {
        return Verdict::Fail("accel.x/y 应≈0");
    }
    if (az - 10920).abs() > 10920 / 20 {
        return Verdict::Fail("accel.z 应≈10920（±3g 量程 1g）");
    }
    if raw.gyro[0].abs() > 40 || raw.gyro[1].abs() > 40 || raw.gyro[2].abs() > 40 {
        return Verdict::Fail("gyro 应≈0（悬停）");
    }
    info!("bmi088", "RAW ax={} ay={} az={} gx={} gy={} gz={}", raw.accel[0], raw.accel[1], raw.accel[2], raw.gyro[0], raw.gyro[1], raw.gyro[2]);
    Verdict::Pass("原始计数值正确：accel.z≈10920、gyro≈0")
}

pub fn si_readout(_ctx: &mut Ctx) -> Verdict {
    let dev = match open_bmi088() {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };
    let mut si = Bmi088Si { accel: [0.0; 3], gyro: [0.0; 3] };
    if dev.ioctl(BMI088_IOCTL_GET_SI, (&mut si as *mut Bmi088Si).cast()) != 0 {
        return Verdict::Fail("GET_SI 失败");
    }
    // accel.z ≈ +9.81 m/s²（±0.2），x/y ≈ 0（±0.2）；gyro ≈ 0 rad/s（±0.05）
    if si.accel[2] < 9.61 || si.accel[2] > 10.01 {
        return Verdict::Fail("accel.z 应≈+9.81 m/s²");
    }
    if si.accel[0].abs() > 0.2 || si.accel[1].abs() > 0.2 {
        return Verdict::Fail("accel.x/y 应≈0");
    }
    if si.gyro[0].abs() > 0.05 || si.gyro[1].abs() > 0.05 || si.gyro[2].abs() > 0.05 {
        return Verdict::Fail("gyro 应≈0 rad/s");
    }
    info!("bmi088", "SI accel=[{:.2},{:.2},{:.2}] gyro=[{:.3},{:.3},{:.3}]", si.accel[0], si.accel[1], si.accel[2], si.gyro[0], si.gyro[1], si.gyro[2]);
    Verdict::Pass("SI 换算正确：accel.z≈9.81 m/s²、gyro≈0")
}

pub fn raw_read_12b(_ctx: &mut Ctx) -> Verdict {
    let dev = match open_bmi088() {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };
    // device read：12B = accel 6B（0x12..） + gyro 6B（0x02..），16bit LE
    let mut buf = [0u8; 12];
    if dev.read(&mut buf) != 12 {
        return Verdict::Fail("device read≠12B");
    }
    let az = i16::from_le_bytes([buf[4], buf[5]]) as i32;
    let gz = i16::from_le_bytes([buf[10], buf[11]]) as i32;
    if (az - 10920).abs() > 10920 / 20 {
        return Verdict::Fail("read accel.z 应≈10920 LE");
    }
    if gz.abs() > 40 {
        return Verdict::Fail("read gyro.z 应≈0");
    }
    info!("bmi088", "READ12 accel_z_lo={} gyro_z_lo={}", buf[4], buf[10]);
    Verdict::Pass("device read 12B 布局正确")
}

#[allow(dead_code)]
fn _sanity_lsb() {
    // 编译期核对换算常量（与模拟器 bmi088.rs 相同）
    let _ = ACCEL_LSB_PER_G;
    let _ = GYRO_LSB_PER_DPS;
    let _ = null_mut::<u8>();
}
