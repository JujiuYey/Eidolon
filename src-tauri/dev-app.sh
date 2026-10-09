#!/usr/bin/env bash
# 组装并启动「Eidolon Dev」开发壳 App。
#
# 背景：macOS 26 按 App 授予「本地网络」权限，cargo 编译出的裸二进制
# （target/debug/app）没有稳定 Bundle 身份，既不弹授权弹窗、也出现在
# 系统设置的本地网络列表里，导致访问内网服务（如禅道）被静默拦截。
# 本脚本把裸二进制包进一个带固定 Bundle ID 的 .app，使其可正常授权。
#
# 用法：
#   pnpm dev              # 先启动 vite（或继续用 pnpm tauri dev 的也行）
#   ./dev-app.sh          # cargo build + 组装 .app + 打开
#   OPEN=0 ./dev-app.sh   # 只组装不打开
#
# 首次启动留意系统弹出的「"Eidolon Dev"想要访问本地网络」弹窗，点「允许」。
# Rust 代码变更后重跑本脚本即可（会重新 cargo build）。

set -euo pipefail

cd "$(dirname "$0")"

echo "==> cargo build"
cargo build

APP_DIR="target/dev-app/Eidolon Dev.app"
mkdir -p "$APP_DIR/Contents/MacOS"

cat > "$APP_DIR/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>dev.eidolon.app.dev</string>
    <key>CFBundleName</key>
    <string>Eidolon Dev</string>
    <key>CFBundleExecutable</key>
    <string>eidolon-dev</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSLocalNetworkUsageDescription</key>
    <string>Eidolon 需要访问本地网络以连接你的禅道实例。</string>
</dict>
</plist>
PLIST

printf 'APPL????' > "$APP_DIR/Contents/PkgInfo"

# 启动器：从 Contents/MacOS 回退四级到 target/，再进 debug/
cat > "$APP_DIR/Contents/MacOS/eidolon-dev" <<'LAUNCH'
#!/usr/bin/env bash
exec "$(dirname "$0")/../../../../debug/app" "$@"
LAUNCH
chmod +x "$APP_DIR/Contents/MacOS/eidolon-dev"

codesign --force --sign - "$APP_DIR"

echo "==> 已生成 $APP_DIR"
if [[ "${OPEN:-1}" == "1" ]]; then
    echo "==> 启动 Eidolon Dev（首次请允许「本地网络」授权弹窗）"
    open "$APP_DIR"
fi
