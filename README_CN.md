# Geektools | 极客工具箱

[English](./README.md) | [中文](./README_CN.md)

## 特别感谢

- [rust](https://www.rust-lang.org/)

- [homebrew](https://brew.sh/zh-cn/)

- [crate.io](https://crates.io/)

## 这是什么?
- 一个运行内置使用shell脚本程序
- 一个运行网络url shell脚本的程序
- 内置OTA，可在发布版本间切换
- 还有一些……

## 如何安装?

### 首先:如果你是Windows系统(最常见),你无法使用,尝试移步「皇帝的未来的项目（敬请期待）」

### 接着，根据你的要求，选择下载我们部署好的的包体或进行编译

### 快速开始
- 运行
    ```bash
    # 安装curl
    # sudo apt install curl 或 sudo yum install curl
    curl "https://raw.githubusercontent.com/PeterFujiyu/geektools/refs/heads/master/install.sh" | bash
    ```
- 使用吧 🎉

### 手动构建(仅在macOS的rust环境中进行过测试,不保证Linux的可用性)
#### 准备工作
- 克隆仓库
```bash
git clone https://github.com/PeterFujiyu/geektools.git
cd geektools
```
- 安装rust运行环境(如果已经安装则无需)
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
echo ". "$HOME/.cargo/env"" >> ~/.bashrc
echo ". "$HOME/.cargo/env"" >> ~/.zshrc
```
- 安装Homebrew(若已安装或无需多平台则无需)
```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```
#### 构建(全平台/本机选其一)
- 构建产物全平台
```bash
sh ./allrelease.sh
# 在项目根/target/disk中
```

- 构建产物本机
```bash
cargo build --release
# 在项目根/target/release/geektools
```

## 贡献指南

1. Fork 本仓库并拉取至本地；
2. 创建分支：`git checkout -b feature/your-feature`；
3. 提交代码：`git commit -m "feat: your feature"`；
4. 推送分支：`git push origin feature/your-feature`；
5. 在 GitHub 上发起 Pull Request。

欢迎提交 Issue 与 PR，一同完善项目！

## 许可证
版权所有 Copyright ©️ PeterFujiyu
[GPLv3 LICENSE](./LICENSE)