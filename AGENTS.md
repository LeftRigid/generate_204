## 命令执行环境

项目位于 Windows 11，Agent 运行于 WSL2，`node_modules` 仅兼容 Windows。

- **WSL 环境**：文件查看/编辑、文本检索（grep/ripgrep/find）等只读或基础系统操作。
- **Windows Interop 执行**（直接调用 Windows 二进制）：
  - **Node/包管理**：所有构建、测试与依赖安装严禁使用 WSL 原生命令，须调用 Windows 工具链（如 `node.exe`, `uv.exe` 或 `pwsh.exe -NoP -c "npm -v"`）。