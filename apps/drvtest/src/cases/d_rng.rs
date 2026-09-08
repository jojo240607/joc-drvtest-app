//! d_rng：真随机数发生器。两次读数应不同且非全 0。
//! 模拟器 rng 为确定性 xorshift32（每次读推进），同样满足"两次不同"。

use crate::cap;
use crate::verdict::{Ctx, Verdict};

/// rng0：连续两次 32 位读数，互不相同。
pub fn rng_read(_ctx: &mut Ctx) -> Verdict {
    let dev = match cap::open("rng0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };

    let mut a = [0u8; 4];
    let mut b = [0u8; 4];
    let rc1 = dev.read(&mut a);
    let rc2 = dev.read(&mut b);
    if rc1 != 4 || rc2 != 4 {
        return Verdict::Fail("rng read 长度不符");
    }
    let va = u32::from_le_bytes(a);
    let vb = u32::from_le_bytes(b);
    if va == 0 && vb == 0 {
        return Verdict::Fail("rng 两次全 0");
    }
    if va == vb {
        return Verdict::Fail("两次读数相同（非随机）");
    }
    Verdict::Pass("rng0 两次读数不同（随机源活跃）")
}
