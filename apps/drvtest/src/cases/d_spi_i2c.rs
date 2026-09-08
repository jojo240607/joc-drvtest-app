//! d_spi_i2c：总线外设（主机侧）。判据：寄存器回读 + 扫描完成。
//! 模拟器/无外设板：i2c 扫描 0 从机、spi 只读 CR1（XFER 需从机应答 → 不测）。

use rtos_app_sdk::ioctl::*;

use crate::cap;
use crate::verdict::{Ctx, Verdict};

/// 镜像 drv/i2c.h i2c_scan_t（130 字节）。
#[repr(C)]
struct I2cScan {
    acks: [u8; 128],
    found: u16,
}

/// i2c0：总线扫描完成 + CCR/CR2_FREQ 寄存器回读非 0。
pub fn i2c_scan(_ctx: &mut Ctx) -> Verdict {
    let dev = match cap::open("i2c0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };

    // 寄存器回读（时钟配置已由 open 完成）。
    let mut ccr: u32 = 0;
    if dev.ioctl(I2C_IOCTL_GET_CCR, (&mut ccr as *mut u32).cast()) != 0 || ccr == 0 {
        return Verdict::Fail("GET_CCR 失败/为 0");
    }
    let mut freq: u32 = 0;
    if dev.ioctl(I2C_IOCTL_GET_CR2_FREQ, (&mut freq as *mut u32).cast()) != 0 || freq == 0 {
        return Verdict::Fail("GET_CR2_FREQ 失败/为 0");
    }

    // 总线扫描：无外挂从机 → found=0（扫描本身完成即证明引擎可跑）。
    let mut scan = I2cScan {
        acks: [0u8; 128],
        found: 0xFFFF,
    };
    if dev.ioctl(I2C_IOCTL_BUS_SCAN, (&mut scan as *mut I2cScan).cast()) != 0 {
        return Verdict::Fail("BUS_SCAN 失败");
    }
    if scan.found > 128 {
        return Verdict::Fail("扫描结果越界");
    }

    Verdict::Pass("i2c0 寄存器回读 + 总线扫描完成")
}

/// spi：任一主机总线 open + CR1 回读非 0 即过（XFER 需从机应答，不测）。
/// 板级引脚占用：spi0(SPI1) 的 MISO=PA6 与 pwm0(TIM3_CH1) 冲突 → open 失败属
/// 平台配置，非驱动缺陷；spi1(SPI2@PI1/2/3)、spi2(SPI3@PB3/4/5) 应可用。
pub fn spi_cr1(_ctx: &mut Ctx) -> Verdict {
    let mut opened = 0;
    for name in ["spi0", "spi1", "spi2"] {
        let dev = match cap::open(name) {
            Ok(d) => d,
            Err(_) => continue, // 该总线引脚被其他外设占用 → 试下一条
        };
        let mut cr1: u32 = 0;
        if dev.ioctl(SPI_IOCTL_GET_CR1, (&mut cr1 as *mut u32).cast()) != 0 || cr1 == 0 {
            return Verdict::Fail("GET_CR1 失败/为 0");
        }
        opened += 1;
    }
    if opened == 0 {
        return Verdict::Skip("无可用 SPI 总线（引脚均被占用）");
    }
    Verdict::Pass("spi 主机 open + CR1 回读正常")
}
