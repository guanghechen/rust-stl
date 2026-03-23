# rstl-queue

`rstl-queue` 提供 `rstl` 的队列相关数据结构与 trait：

- `CircularQueue`
- `PriorityQueue`
- `LinkedDeque`

并导出通用 trait：

- `QueueLike`
- `DequeLike`
- `CircularQueueLike`
- `PriorityQueueLike`

本 crate 聚焦通用抽象与高性能基础容器。

说明：

- `PriorityQueue<T>` 采用 `T: Ord` 的内建比较，不再接收自定义比较器。
- 如需 max-heap 语义，可使用 `PriorityQueue<std::cmp::Reverse<T>>`。
