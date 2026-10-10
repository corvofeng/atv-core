#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

ARCH="${1:-arm64}"

case "$ARCH" in
    arm64)
        RUST_TARGET="aarch64-apple-darwin"
        SWIFT_TARGET="arm64-apple-macos12.0"
        ARCH_DESC="Apple Silicon (arm64 / M1+)"
        ;;
    x86_64)
        RUST_TARGET="x86_64-apple-darwin"
        SWIFT_TARGET="x86_64-apple-macos12.0"
        ARCH_DESC="Intel (x86_64)"
        ;;
    universal)
        ARCH_DESC="Universal Binary (arm64 + x86_64)"
        ;;
    *)
        echo "❌ Unsupported architecture: $ARCH (supported: arm64, x86_64, universal)"
        exit 1
        ;;
esac

echo "=========================================="
echo "🔨 Building Apple TV Remote for macOS (.app) [$ARCH_DESC]"
echo "=========================================="

cd "$ROOT_DIR"

# 1. Compile Rust core binary
BUILD_DIR="$ROOT_DIR/build"
mkdir -p "$BUILD_DIR"

if [ "$ARCH" = "universal" ]; then
    echo "🦀 [1/4] Compiling Rust core engine (atv-cli) as Universal Binary..."
    cargo build --release -p atv-cli --target aarch64-apple-darwin
    cargo build --release -p atv-cli --target x86_64-apple-darwin
    mkdir -p "$ROOT_DIR/target/release"
    lipo -create -output "$ROOT_DIR/target/release/atv-cli" \
        "$ROOT_DIR/target/aarch64-apple-darwin/release/atv-cli" \
        "$ROOT_DIR/target/x86_64-apple-darwin/release/atv-cli"
    ATV_CLI_BIN="$ROOT_DIR/target/release/atv-cli"
    echo "   ✅ Universal Binary created: $(lipo -info "$ATV_CLI_BIN")"
else
    echo "🦀 [1/4] Compiling Rust core engine (atv-cli) for $ARCH..."
    cargo build --release -p atv-cli --target "$RUST_TARGET"
    ATV_CLI_BIN="$ROOT_DIR/target/$RUST_TARGET/release/atv-cli"
    echo "   ✅ Single-arch Binary created: $(file "$ATV_CLI_BIN")"
fi

# 2. Generate App Icon if missing
if [ ! -f "$ROOT_DIR/mac-app/resources/AppIcon.icns" ]; then
    echo "🎨 [2/4] Generating AppIcon.icns..."
    swift "$ROOT_DIR/scripts/generate_icon.swift"
else
    echo "🎨 [2/4] AppIcon.icns already exists."
fi

# 3. Compile Swift Native App
echo "🍏 [3/4] Compiling Swift Native App for $ARCH..."
SWIFT_BIN="$BUILD_DIR/AppleTVRemote"

if [ "$ARCH" = "universal" ]; then
    swiftc -O -target arm64-apple-macos12.0 \
        "$ROOT_DIR"/mac-app/src/*.swift \
        -o "${SWIFT_BIN}_arm64"
    swiftc -O -target x86_64-apple-macos12.0 \
        "$ROOT_DIR"/mac-app/src/*.swift \
        -o "${SWIFT_BIN}_x86_64"
    lipo -create -output "$SWIFT_BIN" "${SWIFT_BIN}_arm64" "${SWIFT_BIN}_x86_64"
    rm -f "${SWIFT_BIN}_arm64" "${SWIFT_BIN}_x86_64"
    echo "   ✅ Universal Swift Binary created: $(lipo -info "$SWIFT_BIN")"
else
    swiftc -O -target "$SWIFT_TARGET" \
        "$ROOT_DIR"/mac-app/src/*.swift \
        -o "$SWIFT_BIN"
    echo "   ✅ Swift Binary created: $(file "$SWIFT_BIN")"
fi

# 4. Assemble macOS .app Bundle
APP_BUNDLE="$BUILD_DIR/AppleTVRemote.app"
echo "📦 [4/4] Assembling $APP_BUNDLE..."
rm -rf "$APP_BUNDLE"

mkdir -p "$APP_BUNDLE/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Resources"

# Copy binaries
cp "$SWIFT_BIN" "$APP_BUNDLE/Contents/MacOS/AppleTVRemote"
cp "$ATV_CLI_BIN" "$APP_BUNDLE/Contents/MacOS/atv-cli"
chmod +x "$APP_BUNDLE/Contents/MacOS/AppleTVRemote"
chmod +x "$APP_BUNDLE/Contents/MacOS/atv-cli"

# Copy metadata and resources
cp "$ROOT_DIR/mac-app/Info.plist" "$APP_BUNDLE/Contents/Info.plist"
echo "APPL????" > "$APP_BUNDLE/Contents/PkgInfo"
cp "$ROOT_DIR/mac-app/resources/AppIcon.icns" "$APP_BUNDLE/Contents/Resources/AppIcon.icns"
# Translations of Info.plist strings (the Accessibility permission prompt)
cp -R "$ROOT_DIR"/mac-app/resources/*.lproj "$APP_BUNDLE/Contents/Resources/"

# Clean temporary standalone swift binary
rm -f "$SWIFT_BIN"

# 5. Code sign application bundle (Auto-detecting best certificate)
find_best_identity() {
    # 1. 优先使用环境变量指定的证书
    if [ -n "$CODESIGN_IDENTITY" ]; then
        if security find-identity -p codesigning | grep -q "\"$CODESIGN_IDENTITY\""; then
            echo "$CODESIGN_IDENTITY"
            return
        fi
    fi

    # 2. 匹配本地通用持久化证书 (Corvo Development) - 优先保证本地权限持久不失效
    if security find-identity -p codesigning | grep -q "\"Corvo Development\""; then
        echo "Corvo Development"
        return
    fi

    # 3. 兼容历史项目自签证书 (AppleTVRemote Development)
    if security find-identity -p codesigning | grep -q "\"AppleTVRemote Development\""; then
        echo "AppleTVRemote Development"
        return
    fi

    # 4. 匹配 Apple 官方发布证书 (Developer ID Application)
    local dev_id
    dev_id=$(security find-identity -v -p codesigning | grep "Developer ID Application:" | head -n1 | sed -E 's/.*"([^"]+)".*/\1/')
    if [ -n "$dev_id" ]; then
        echo "$dev_id"
        return
    fi

    # 5. 匹配 Apple 官方开发证书 (Apple Development)
    local apple_dev
    apple_dev=$(security find-identity -v -p codesigning | grep "Apple Development:" | head -n1 | sed -E 's/.*"([^"]+)".*/\1/')
    if [ -n "$apple_dev" ]; then
        echo "$apple_dev"
        return
    fi

    echo ""
}

SIGN_IDENTITY=$(find_best_identity)

if [ -z "$SIGN_IDENTITY" ]; then
    echo "=========================================="
    echo "❌ 致命错误: 未检测到任何可用的代码签名证书！"
    echo "已终止构建。请运行 ./scripts/setup_certificate.sh 配置证书，或检查 CI Secret。"
    echo "当前钥匙串中可用身份如下:"
    security find-identity -p codesigning || true
    echo "=========================================="
    exit 1
fi

echo "🔏 Signing application with certificate: \"$SIGN_IDENTITY\"..."
codesign --force --sign "$SIGN_IDENTITY" --identifier "com.corvofeng.AppleTVRemote.helper" "$APP_BUNDLE/Contents/MacOS/atv-cli"
codesign --force --sign "$SIGN_IDENTITY" --identifier "com.corvofeng.AppleTVRemote" "$APP_BUNDLE/Contents/MacOS/AppleTVRemote"
codesign --force --sign "$SIGN_IDENTITY" "$APP_BUNDLE"
echo "✨ Signed successfully with \"$SIGN_IDENTITY\"! Permissions will survive rebuilds."

echo "🔍 验证代码签名详情:"
codesign -dv --verbose=2 "$APP_BUNDLE" 2>&1 | grep -E "(Authority=|Signature=|Identifier=)" || true

echo "=========================================="
echo "✅ Successfully built: $APP_BUNDLE"
echo "=========================================="

# ⚠️  Self-signed certificate distribution warning
if echo "$SIGN_IDENTITY" | grep -qE "(Corvo Development|AppleTVRemote Development)"; then
    echo ""
    echo "⚠️  【跨机器分发提示】"
    echo "当前使用的是本机自签名证书: \"$SIGN_IDENTITY\""
    echo "该证书仅在本机钥匙串中受信任。将 App 分发到另一台 Mac 时，"
    echo "对方 macOS Gatekeeper 会拒绝启动（显示「无法验证」错误）。"
    echo ""
    echo "解决方案 (任选其一):"
    echo "  方案 A — 在目标 Mac 上导入证书 (推荐用于私人分发):"
    echo "    ./scripts/import_certificate.sh Corvo_Development.p12"
    echo ""
    echo "  方案 B — 目标 Mac 上跳过 Gatekeeper (一次性):"
    echo "    xattr -dr com.apple.quarantine /Applications/AppleTVRemote.app"
    echo "    或: 右键 App -> 打开 -> 点击「打开」"
    echo ""
    echo "  方案 C — 使用 Apple Developer 账号正式签名 (用于公开分发):"
    echo "    CODESIGN_IDENTITY=\"Apple Development: you@example.com\" ./scripts/build_app.sh"
    echo "=========================================="
fi
