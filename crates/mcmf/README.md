# rstl-mcmf

`rstl-mcmf` 提供 `Mcmf`（Min-Cost Max-Flow）算法实现。

核心导出：

- `Mcmf`
- `McmfEdge`
- `McmfResult`
- `McmfLike`

主要 API：

- TS 风格接口：`init/add_edge/min_cost_max_flow/mincut`
- Checked 接口：`try_init/try_add_edge`
- 可配置最短路策略：`McmfShortestPathStrategy::{Auto, Spfa}`

前置条件：

- `n > 0`
- `source < n && sink < n`
- `source != sink`
- `cap >= 0`
- 残量网络中不应存在可无限改进的负环

复杂度：

- 单次 `min_cost_max_flow`：
  - `Auto`：`O(V * E + F * E * log V)`（首轮 SPFA/Bellman-Ford 初始化 potential，后续 Dijkstra 增广）
  - `Spfa`：`O(F * V * E)`（SPFA/Bellman-Ford 增广版常见上界）
- 每条 `add_edge`：`O(1)`
- `mincut`：`O(E)`（在求解完成后扫描边）

行为说明：

- `Auto` 模式采用 residual graph + potential + Dijkstra；`Spfa` 模式采用 SPFA/Bellman-Ford。
- `min_cost_max_flow` 带缓存：若图未修改，重复调用直接返回上次结果。
- `mincut` 返回严格最小割边集合：`u` 在残量网络中从 `source` 可达、`v` 不可达，且 `cap(u,v) > 0`。
