//! d_i2s：I2S 主机 TX 配置面（i2s0 = I2S2 @ PB12/13/15，48kHz）。
//!
//! Discovery 板未接外部 codec，i2s0 只跑片上 I2S 逻辑（PLLI2S 时钟 + 预分频 +
//! 数据移位），无音频回路。判据：open 后 I2SE 使能、请求采样率 48kHz、
//! PLLI2S 输出（I2SxCLK）非 0 且就绪——时钟链路数值正确。

use rtos_app_sdk::ioctl::*;

use crate::cap;
use crate::verdict::{Ctx, Verdict};

/// i2s0：I2SE 使能 + 48kHz + PLLI2S 时钟链路回读。
pub fn i2s_config(_ctx: &mut Ctx) -> Verdict {
    let dev = match cap::open("i2s0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };

    // I2SE（I2SCFGR bit10）应已使能（open 即启动主机 TX）
    let mut cfgr: u32 = 0;
    if dev.ioctl(I2S_IOCTL_GET_I2SCFGR, (&mut cfgr as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_I2SCFGR 失败");
    }
    if cfgr & I2S_I2SCFGR_I2SE == 0 {
        return Verdict::Fail("I2SE 未使能");
    }

    let mut hz: u32 = 0;
    if dev.ioctl(I2S_IOCTL_GET_AUDIO_HZ, (&mut hz as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_AUDIO_HZ 失败");
    }
    if hz != 48000 {
        return Verdict::Fail("请求采样率 ≠ 48kHz");
    }

    let mut clk: u32 = 0;
    if dev.ioctl(I2S_IOCTL_GET_I2S_CLK, (&mut clk as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_I2S_CLK 失败");
    }
    if clk == 0 {
        return Verdict::Fail("I2SxCLK（PLLI2S 输出）为 0");
    }

    let mut rdy: u32 = 0;
    if dev.ioctl(I2S_IOCTL_GET_PLL_RDY, (&mut rdy as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_PLL_RDY 失败");
    }
    if rdy == 0 {
        return Verdict::Fail("PLLI2SRDY 未就绪");
    }

    Verdict::Pass("i2s0 I2SE 使能 + 48kHz + PLLI2S 时钟链路正常")
}
