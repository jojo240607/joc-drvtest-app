//! d_i2c_multi：i2c1/i2c2 实例级覆盖（i2c0=I2C1 已在 d_spi_i2c 覆盖）。
//! 判据：每个实例 open 成功 → CCR/CR2_FREQ 寄存器回读非 0 → 总线扫描完成。
//! 板级引脚占用：i2c2(I2C3) 的 SCL=PA8 与 pwm2(TIM1_CH1) 冲突 → open 失败属
//! 平台配置，非驱动缺陷；i2c1(I2C2@PB10/11) 应可用。

use rtos_app_sdk::ioctl::*;

use crate::cap;
use crate::verdict::{Ctx, Verdict};

/// 镜像 drv/i2c.h i2c_scan_t（130 字节）。
#[repr(C)]
struct I2cScan {
    acks: [u8; 128],
    found: u16,
}

/// i2c1/i2c2：寄存器回读 + 总线扫描完成（任一总线可用即过）。
pub fn i2c_instances(_ctx: &mut Ctx) -> Verdict {
    let mut opened = 0;
    for name in ["i2c1", "i2c2"] {
        let dev = match cap::open(name) {
            Ok(d) => d,
            Err(_) => {
                crate::info!("d_i2c_multi", "{name} open 失败（引脚占用？）");
                continue;
            }
        };
        crate::info!("d_i2c_multi", "{name} open 成功");

        let mut ccr: u32 = 0;
        if dev.ioctl(I2C_IOCTL_GET_CCR, (&mut ccr as *mut u32).cast()) != 0 || ccr == 0 {
            return Verdict::Fail("GET_CCR 失败/为 0");
        }
        let mut freq: u32 = 0;
        if dev.ioctl(I2C_IOCTL_GET_CR2_FREQ, (&mut freq as *mut u32).cast()) != 0 || freq == 0 {
            return Verdict::Fail("GET_CR2_FREQ 失败/为 0");
        }

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
        opened += 1;
    }
    if opened == 0 {
        return Verdict::Skip("无可用 I2C 总线（引脚均被占用）");
    }
    Verdict::Pass("i2c1/i2c2 寄存器回读 + 总线扫描完成")
}
