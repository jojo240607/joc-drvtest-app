#!/usr/bin/env python3
"""joc-rtos-app-sdk 阶段 2 独立构建：产出可烧录到 APP_FLASH 块的 app.bin。

流程：
  1) cargo build --release（workspace 默认 target thumbv7em-none-eabihf）
     -> target/thumbv7em-none-eabihf/release/lib{app}.a
  2) arm-none-eabi-gcc -nostartfiles -T linker/app.ld 链接 -> app.elf
     （app.ld 在头部生成 app_header_t：magic/abi_version/entry）
  3) arm-none-eabi-objcopy -O binary app.elf app.bin

用法：
  python build_app.py                 # 默认构建 apps/demo
  python build_app.py --app apps/demo # 指定应用 crate
  python build_app.py --check         # 仅校验工具链
"""
import subprocess, sys, os, shutil, argparse

ROOT = os.path.dirname(os.path.abspath(__file__))
DEFAULT_APP = "apps/demo"
APP_FLASH_BUDGET = 384 * 1024  # APP_FLASH 384KB

def run(cmd, cwd=None):
    print("+", " ".join(cmd), flush=True)
    subprocess.check_call(cmd, cwd=cwd or ROOT)

def check_tools():
    ok = True
    for t in ("cargo", "arm-none-eabi-gcc", "arm-none-eabi-objcopy"):
        if shutil.which(t) is None:
            print(f"[ERR] 工具未找到: {t}（请加入 PATH）", file=sys.stderr)
            ok = False
    return ok

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--app", default=DEFAULT_APP, help="应用 crate 目录（含 Cargo.toml）")
    ap.add_argument("--check", action="store_true", help="仅校验工具链")
    ap.add_argument("--out", default="app.bin", help="输出 app.bin 路径")
    args = ap.parse_args()

    if not check_tools():
        sys.exit(1)
    if args.check:
        print("[OK] 工具链就绪")
        return

    app_dir = os.path.join(ROOT, args.app)
    cargo_toml = os.path.join(app_dir, "Cargo.toml")
    if not os.path.isfile(cargo_toml):
        print(f"[ERR] 未找到应用 Cargo.toml: {app_dir}", file=sys.stderr)
        sys.exit(1)

    # 1) cargo -> libapp.a（workspace 根构建）；包名从 Cargo.toml 读取
    pkg = None
    for line in open(cargo_toml):
        line = line.strip()
        if line.startswith("name"):
            pkg = line.split("=", 1)[1].strip().strip('"').strip("'")
            break
    if not pkg:
        print(f"[ERR] 无法从 {cargo_toml} 读取包名", file=sys.stderr)
        sys.exit(1)
    libname = pkg.replace("-", "_")
    run(["cargo", "build", "--release", "-p", pkg])
    libapp = os.path.join(ROOT, "target", "thumbv7em-none-eabihf", "release", f"lib{libname}.a")
    if not os.path.exists(libapp):
        print(f"[ERR] 未找到 {libapp}", file=sys.stderr)
        sys.exit(1)

    # 2) 链接（app.ld 生成头部 + 定位到 APP_FLASH/APP_RAM）
    app_elf = os.path.join(ROOT, "app.elf")
    run(["arm-none-eabi-gcc", "-nostartfiles", "-T", os.path.join(ROOT, "linker", "app.ld"),
         "-Wl,--gc-sections", "-Wl,--no-warn-rwx-segments",
         "-o", app_elf, libapp, "-lgcc"])

    # 3) objcopy -> app.bin
    app_bin = os.path.join(ROOT, args.out)
    run(["arm-none-eabi-objcopy", "-O", "binary", app_elf, app_bin])
    sz = os.path.getsize(app_bin)
    print(f"[OK] app.bin 产出: {app_bin} ({sz} bytes, 须 < {APP_FLASH_BUDGET})")
    if sz > APP_FLASH_BUDGET:
        print(f"[ERR] App 镜像超过 APP_FLASH {APP_FLASH_BUDGET // 1024}K 预算", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
