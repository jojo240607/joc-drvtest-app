//! 各任务独立栈（App RAM `.rust_bss` 段；系统加载器清零）。
#![allow(static_mut_refs)]

/// 驱动验证运行器任务栈（跑全部用例；8KB 富余）。
#[link_section = ".rust_bss"]
pub static mut RUNNER_STACK: [u8; 8192] = [0u8; 8192];

/// 心跳任务栈（低优先级，测试期间证明调度活着）。
#[link_section = ".rust_bss"]
pub static mut HB_STACK: [u8; 2048] = [0u8; 2048];

/// k_sdk 冒烟用例的 worker 任务栈。
#[link_section = ".rust_bss"]
pub static mut WORKER_STACK: [u8; 2048] = [0u8; 2048];
