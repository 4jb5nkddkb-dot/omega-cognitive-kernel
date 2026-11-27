# 可移植性摘要
> **语言与基准说明**：本文档为可移植性中文摘要，权威矩阵与最新平台测试状态以英文 [`HOST_PORTABILITY.md`](HOST_PORTABILITY.md) 为准。

**Last updated:** 2026-11-26
**Target source version:** 0.6.0

本文档是中文概览；权威构建命令和验证矩阵见 [HOST_PORTABILITY.md](./HOST_PORTABILITY.md)。

## 目标

核心认知循环（L0 / L1 / L2 + 状态持久化抽象）可在以下环境运行：

- 原生 `std`（当前默认）
- `no_std` + `alloc`
- WASM（browser / wasmtime）
- 嵌入式（未来，通过宿主提供 storage / time / uuid）

## 当前状态

| 目标              | 状态       | 备注                                      |
|-------------------|------------|-------------------------------------------|
| no_std core       | 基本完成   | portable UUID / time helpers 已加入       |
| WASM runtime smoke| CI 已覆盖  | 见 `scripts/test-wasm-runtime.sh`         |
| Host-gated std    | 已实现     | 文件 IO、网络、CLI 通过 feature 隔离      |
| MSRV 1.74         | 已锁定     | 依赖图已 pin，Clippy 在 1.74 通过         |
| Dashboard / CLI   | 仅 std     | 依赖 `std` + `gui` feature                |

## 已知限制

- 完整 Dashboard 与 CLI 仍依赖 `std`。
- 文件持久化在 `no_std` 下需要宿主存储适配层；当前 crate 尚未导出该 trait。
- HTTP adapter、tracing-subscriber 等仅在 `std` 下可用。
- GUI（egui/eframe）不支持 no_std / WASM 桌面。

## 最小接入示例（no_std 宿主，拟议接口）

```rust
extern crate alloc;
use alloc::vec::Vec;

#[derive(Debug)]
pub struct StorageError;

/// 宿主可实现的最小持久化接口（no_std 友好）。
///
/// 这是宿主适配层的拟议接口，不是当前 crate 已导出的 trait。
pub trait HostStorage {
    fn load(&self) -> Option<Vec<u8>>;
    fn save(&self, data: &[u8]) -> Result<(), StorageError>;
}
```

## CI 覆盖

- `.github/workflows/no-std.yml`
- `.github/workflows/scheduled-portability.yml`
- WASM smoke 脚本

## 下一步

- [ ] 补一份完整的宿主 `HostStorage` trait 文档与示例
- [x] 增加 `thumbv7em-none-eabihf` 嵌入式目标的编译检查（`scripts/test-no-std.sh`）
