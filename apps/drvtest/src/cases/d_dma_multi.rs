//! d_dma_multi：dma2 控制器实例级覆盖（dma1 已在 d_dma 覆盖）。
//! DMA 设备无公开 ioctl 命令面，判据：open 成功（控制器可访问、流池管理器
//! 就绪）。DMA2 在 F407 上服务于 SPI1/TIM1/TIM8 等外设请求。

use crate::cap;
use crate::verdict::{Ctx, Verdict};

/// dma2：open 成功（流池管理器可访问）。
pub fn dma2_pool(_ctx: &mut Ctx) -> Verdict {
    match cap::open("dma2") {
        Ok(_) => Verdict::Pass("dma2 池管理器 open 成功"),
        Err(r) => Verdict::Skip(r),
    }
}
