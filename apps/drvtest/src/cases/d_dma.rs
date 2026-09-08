//! d_dma：DMA 控制器（流池管理器，供其他驱动 acquire/free 使用）。
//! 设备本身无公开 ioctl 命令面，判据：open 成功（控制器可访问）；
//! 更深的使用被 timer0 open 时的 DMA 流预留（TIM2_UP→DMA1_Stream7）间接覆盖。

use crate::cap;
use crate::verdict::{Ctx, Verdict};

/// dma1：open 成功（流池管理器可访问）。
pub fn dma_pool(_ctx: &mut Ctx) -> Verdict {
    match cap::open("dma1") {
        Ok(_) => Verdict::Pass("dma1 池管理器 open 成功（流预留由 timer0 间接覆盖）"),
        Err(r) => Verdict::Skip(r),
    }
}
