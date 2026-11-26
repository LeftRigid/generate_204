## 命令执行环境

项目位于 Windows 11，Agent 运行于 WSL2，项目使用 Windows Rust 工具链。

- **WSL 环境**：文件查看/编辑、文本检索（grep/ripgrep/find）等只读或基础系统操作。
- **Windows Interop 执行**（直接调用 Windows 二进制）：
  - **Rust/Cargo**：构建、测试与依赖管理均通过 Windows 的 `cargo.exe` 执行，例如 `cargo.exe build`、`cargo.exe test` 或 `pwsh.exe -NoP -c "cargo build"`。