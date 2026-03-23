# rstl-dinic

`rstl-dinic` 提供 `Dinic` 最大流算法实现。

核心导出：

- `Dinic`
- `DinicEdge`
- `DinicLike`

前置条件：

- `n > 0`
- `source < n && sink < n`
- `source != sink`
- `cap >= 0`

复杂度：

- 单次 `maxflow`：`O(V^2 * E)`（一般图的经典上界）
- 每条 `add_edge`：`O(1)`
- `mincut`：`O(E)`（在 `maxflow` 之后扫描边）

行为说明：

- 采用 residual graph + level graph + blocking flow。
- `maxflow` 带缓存：若图未修改，重复调用直接返回上次结果。
- `mincut` 返回严格最小割边集合：`u` 在残量网络中从 `source` 可达、`v` 不可达，且 `cap(u,v) > 0`。
