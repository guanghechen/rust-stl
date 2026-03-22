# rstl-stack

`rstl-stack` 提供 `rstl` 的栈相关数据结构与 trait：

- `CircularStack`

并导出通用 trait：

- `StackLike`
- `CircularStackLike`

`CircularStack` 采用固定容量循环缓冲区实现：

- `push/pop/top` 为 `O(1)`
- `resize/rearrange/retain` 为 `O(N)`
- 缩容时会保留最新元素（靠近 top 的元素）
