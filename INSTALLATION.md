# Mole GUI 安装指南

## ️ 重要提示：首次安装需要手动授权

由于本项目未购买 Apple Developer 账号（$99/年），GitHub CI 构建的 DMG **未经过代码签名和公证**，macOS Gatekeeper 会阻止直接安装。

这不是软件有问题，而是 macOS 的安全机制！请按照以下步骤完成安装。

---

##  下载与安装步骤

### 方法 1：右键打开（推荐，最简单）✅

1. **从 [GitHub Releases](https://github.com/tw93/Mole-GUI/releases) 下载最新的 DMG 文件**
   - Apple Silicon (M1/M2/M3): `MoleGui_x.x.x_aarch64.dmg`
   - Intel (x86_64): `MoleGui_x.x.x_x64.dmg`

2. **右键点击 DMG 文件** → 选择 `"Open"`（打开）

3. **系统弹出警告对话框时**，点击 `"Open"`（而不是 "Move to Trash"）

4. **输入管理员密码确认**（如果需要）

5. **将 MoleGui.app 拖拽到 Applications 文件夹**（右侧蓝色文件夹图标）

6. **首次运行时可能还会弹出类似警告**，同样点击 `"Open"` 即可

7. ✅ **之后就可以正常使用了，不会再有警告！**

![安装示意图](https://user-images.githubusercontent.com/example/install-guide.png)

---

### 方法 2：命令行绕过（高级用户）

如果你熟悉终端操作，可以使用以下命令：

```bash
# 下载 DMG 后挂载
hdiutil attach MoleGui_x.x.x_aarch64.dmg

# 复制到 Applications 目录
cp -R /Volumes/MoleGui/MoleGui.app /Applications/

# 移除 quarantine 属性（绕过 Gatekeeper）
xattr -rd com.apple.quarantine /Applications/MoleGui.app

# 卸载 DMG
hdiutil detach /Volumes/MoleGui

# 启动应用
open /Applications/MoleGui.app
```

或者临时禁用 Gatekeeper（不推荐长期使用）：

```bash
sudo spctl --master-disable
```

> ⚠️ **注意**：禁用 Gatekeeper 会降低系统安全性，建议仅在测试时使用，完成后重新启用：
> ```bash
> sudo spctl --master-enable
> ```

---

## ❓ 常见问题

### Q1: 为什么会出现"无法打开，因为来自身份不明的开发者"警告？

**A:** 这是 macOS Gatekeeper 的安全机制。要消除此警告，应用需要经过：
1. **代码签名**（使用 Apple Developer 证书）
2. **公证（Notarization）**（上传到 Apple 服务器验证）

这两个步骤都需要付费的 Apple Developer Program ($99/年)。

对于个人项目或内部测试，当前的免费方案完全够用！只需要首次运行时手动授权一次即可。

---

### Q2: 每次运行都要手动授权吗？

**A:** 不需要！只需在**首次安装和首次运行时**手动授权一次，之后就可以正常使用，不会再有警告。

---

### Q3: 我是开发者，如何本地构建带签名的版本？

**A:** 如果你有 Apple 开发者证书，可以克隆本仓库后本地构建：

```bash
# 1. 克隆仓库
git clone https://github.com/tw93/Mole-GUI.git
cd Mole-GUI

# 2. 安装依赖
pnpm install

# 3. 配置你的开发者证书（编辑 tauri-gui/tauri.conf.json）
# 将 signingIdentity 改为你自己的证书名称
# 例如："Apple Development: your-email@example.com (TEAMID)"

# 4. 构建
pnpm tauri build

# 5. 生成的 DMG 位于：
# tauri-gui/target/release/bundle/dmg/MoleGui_x.x.x_aarch64.dmg
```

---

### Q4: 这个软件安全吗？

**A:** 是的！本项目是开源的，你可以：
- 查看源代码：[GitHub 仓库](https://github.com/tw93/Mole-GUI)
- 自行编译验证
- 提交 Issue 或 PR

Gatekeeper 警告只是因为缺少官方签名，不代表软件不安全。

---

## ️ 技术细节

### 为什么 GitHub CI 不自动签名？

GitHub Actions 环境中没有你的 Apple 开发者证书和私钥。要自动签名并公证，需要配置：

- `APPLE_CERTIFICATE`: base64 编码的 .p12 证书文件
- `APPLE_CERTIFICATE_PASSWORD`: 证书密码
- `APPLE_ID`: Apple Developer 账号邮箱
- `APPLE_PASSWORD`: App Store Connect API Key
- `APPLE_TEAM_ID`: Team ID

这些敏感信息不能公开在代码仓库中，需要作为 GitHub Secrets 配置。

### 本地开发环境已配置签名

本项目已在 `tauri-gui/tauri.conf.json` 中配置了本地开发证书签名。如果你在本地构建，会自动使用该证书签名应用。

---

## 📞 需要帮助？

如果安装过程中遇到问题，请：

1. 检查是否按照上述步骤操作
2. 查看 [GitHub Issues](https://github.com/tw93/Mole-GUI/issues) 是否有类似问题
3. 提交新的 Issue，附上你的 macOS 版本和错误截图

---

## 🙏 感谢

感谢理解和支持！虽然我们无法提供付费签名，但我们会持续改进软件质量，为用户提供更好的体验。

如果你觉得这个项目有用，欢迎给个 ⭐ Star！
