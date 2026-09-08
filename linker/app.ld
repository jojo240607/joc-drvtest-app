/* joc-rtos-app-sdk 独立链接脚本（阶段 2 应用分区）——把 cargo 产出的 libapp.a
 * 链接成可直接烧录到 APP_FLASH 块的 app.bin。
 * 契约（须与 joc-base 系统侧严格一致）：
 *   - APP_FLASH ORIGIN = 0x08060000（sector 7），header 必须在此起点。
 *   - 头部 app_header_t 由本脚本用 LONG() 直接生成：
 *       magic=0x41504800("APH\0") / abi_version=RTOS_ABI_VERSION /
 *       entry=ABSOLUTE(rust_app_start) / app_size=0(占位) / reserved=0
 *   - App 代码 XIP 运行（.app_code 在 APP_FLASH，绝对地址），无需重定位。
 *   - App 运行期 .data 在 APP_RAM（LMA 在 APP_FLASH，由 rust_app_start 自拷贝初值）；
 *     .bss 由系统加载器清零（APP_RAM 区）。
 *   - 该 ORIGIN/LENGTH 必须与 joc-base 链接脚本 APP_RAM 的注入值严格一致
 *     （发布版 RTOS_SELFTEST=OFF: 0x20004000 / 0x1BC00）。
 *   - g_app_slot 由系统在 APP_SLOT_RAM 固定地址定义（发布版 0x2001FC00），
 *     App 只引用不定义，PROVIDE 提供该符号地址。 */

MEMORY
{
  APP_FLASH (rx) : ORIGIN = 0x08060000, LENGTH = 384K
  APP_RAM   (xrw): ORIGIN = 0x20004000, LENGTH = 0x1BC00  /* 发布版：0x20004000..0x2001FC00 (约111KB)；须与 joc-base APP_RAM 注入值一致 */
}

/* 入口固定为 Rust 挂载点；header.entry 由链接器填其绝对地址 */
ENTRY(rust_app_start)

SECTIONS
{
  /* g_app_slot 是系统侧固定地址(0x2001FC00)的服务表，App 只引用不定义。
   * App 独立链接时由 PROVIDE 提供该符号地址，运行时系统已在那里填好函数指针。 */
  PROVIDE(g_app_slot = 0x2001FC00);

  /* ---- 应用头部（固定 32 字节，App 镜像起点）---- */
  .app_header :
  {
    LONG(0x41504800)                    /* magic: "APH\0" */
    LONG(1)                             /* abi_version: MUST == RTOS_ABI_VERSION (joc-base) */
    LONG(ABSOLUTE(rust_app_start))      /* entry: App 入口绝对地址 */
    LONG(0)                             /* app_size: 占位（系统侧仅作日志，不强制校验） */
    LONG(0)                             /* reserved */
    LONG(0)                             /* reserved */
    LONG(0)                             /* reserved */
    LONG(0)                             /* reserved */
    . = ALIGN(16);
  } >APP_FLASH

  /* ---- App 代码 + 只读数据（XIP）---- */
  .app_code :
  {
    . = ALIGN(4);
    *(.text)
    *(.text.*)
    *(.rodata)
    *(.rodata.*)
    *(.glue_7)
    *(.glue_7t)
    *(.ARM.extab* .gnu.linkonce.armextab.*)
    . = ALIGN(4);
  } >APP_FLASH

  /* ---- App 运行期 .data（非零初值；LMA=Flash，由 rust_app_start 自拷贝到 RAM）---- */
  .app_data :
  {
    . = ALIGN(4);
    _sappdata = .;
    *(.rust_data)
    *(.rust_data.*)
    *(.data)
    *(.data.*)
    . = ALIGN(4);
    _eappdata = .;
    _appdata_lma = LOADADDR(.app_data);
  } >APP_RAM AT> APP_FLASH

  /* ---- App 运行期 .bss（全零初值，如栈缓冲；系统加载器清零）---- */
  .app_bss (NOLOAD) :
  {
    . = ALIGN(4);
    _sappbss = .;
    /* 隔离 gap：.app_bss 的栈缓冲可能向下溢出，而 .app_data 恰在其下。
     * 留 0x100 冗余区，即使任务栈意外溢出也不至于立刻踩坏 .app_data 全局。 */
    . = . + 0x100;
    . = ALIGN(4);
    *(.rust_bss)
    *(.rust_bss.*)
    *(.bss)
    *(.bss.*)
    *(COMMON)
    . = ALIGN(4);
    _eappbss = .;
  } >APP_RAM

  /DISCARD/ :
  {
    *(.note .note.*)
    *(.eh_frame)
  }
}
