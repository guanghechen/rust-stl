# rstl-history

`rstl-history` 提供 `rstl` 的历史状态容器：

- `History`

并导出通用 trait：

- `HistoryLike`

`History` 基于 `CircularStack` 实现固定容量历史记录，主要语义：

- `push/backward/forward/go/present/top`：`O(1)`
- `clear`：`O(N)`，但会保留当前 top 作为唯一历史
- `fork`：`O(N)`（深拷贝快照）
- `rearrange`：`O(N)`，并在过滤后稳定维护 `present` 索引

该实现与 `algorithm.ts/history` 主要行为对齐，并修复了 `rearrange` 时 `present` 可能异常跳转的问题。
