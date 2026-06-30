#!/bin/bash
# 版本管理脚本：从 Git tag 提取版本号并同步到所有配置文件
# 用法：./scripts/update-version.sh [version]
# 如果不传参数，则从最新的 Git tag 自动提取版本号

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 获取版本号：优先使用传入的参数，否则从 Git tag 提取
get_version() {
    if [ -n "$1" ]; then
        VERSION="$1"
        log_info "使用指定的版本号: $VERSION"
    else
        # 从最新的 Git tag 提取版本号（去掉 v 前缀）
        LATEST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "")
        if [ -z "$LATEST_TAG" ]; then
            log_error "未找到 Git tag，请确保至少有一个 tag（如 v1.0.0）"
            exit 1
        fi
        
        # 去掉 v 前缀
        VERSION="${LATEST_TAG#v}"
        log_info "从 Git tag '$LATEST_TAG' 提取版本号: $VERSION"
    fi
    
    # 验证版本号格式（语义化版本 x.y.z）
    if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        log_error "无效的版本号格式: $VERSION (期望格式: x.y.z)"
        exit 1
    fi
}

# 更新 package.json 中的版本号
update_package_json() {
    local file="$PROJECT_ROOT/package.json"
    
    if [ ! -f "$file" ]; then
        log_error "找不到文件: $file"
        return 1
    fi
    
    # 使用 sed 更新 version 字段
    sed -i '' "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" "$file"
    log_info "已更新 package.json 版本号为: $VERSION"
}

# 更新 Cargo.toml 中的版本号
update_cargo_toml() {
    local file="$PROJECT_ROOT/tauri-gui/Cargo.toml"
    
    if [ ! -f "$file" ]; then
        log_error "找不到文件: $file"
        return 1
    fi
    
    # 使用 sed 更新 version 字段
    sed -i '' "s/^version = \"[^\"]*\"/version = \"$VERSION\"/" "$file"
    log_info "已更新 Cargo.toml 版本号为: $VERSION"
}

# 更新 tauri.conf.json 中的版本号
update_tauri_conf() {
    local file="$PROJECT_ROOT/tauri-gui/tauri.conf.json"
    
    if [ ! -f "$file" ]; then
        log_error "找不到文件: $file"
        return 1
    fi
    
    # 使用 sed 更新 version 字段
    sed -i '' "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" "$file"
    log_info "已更新 tauri.conf.json 版本号为: $VERSION"
}

# 主函数
main() {
    log_info "========================================="
    log_info "Mole GUI 版本管理工具"
    log_info "========================================="
    
    # 获取版本号
    get_version "$1"
    
    # 检查是否在 Git 仓库中
    if ! git rev-parse --is-inside-work-tree > /dev/null 2>&1; then
        log_error "当前目录不是 Git 仓库"
        exit 1
    fi
    
    # 依次更新各个配置文件
    update_package_json
    update_cargo_toml
    update_tauri_conf
    
    log_info "========================================="
    log_info "✅ 所有配置文件版本号已同步为: $VERSION"
    log_info "========================================="
    log_info ""
    log_info "下一步操作："
    log_info "1. 提交更改: git add package.json tauri-gui/Cargo.toml tauri-gui/tauri.conf.json"
    log_info "2. 创建标签: git tag v$VERSION"
    log_info "3. 推送标签: git push origin v$VERSION"
    log_info "4. GitHub Actions 将自动构建并发布 Release"
}

# 执行主函数
main "$@"
