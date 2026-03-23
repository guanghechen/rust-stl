# rstl-permutation

`rstl-permutation` 提供 `STL-style` 的排列算法：

- `next_permutation`
- `prev_permutation`
- `permutation_indices`
- `for_each_permutation`

本 crate 聚焦通用、可组合的排列能力：

- 对任意 `T: Ord` 切片原地计算 next/prev permutation。
- 提供按区间生成数字排列的可复用 buffer 游标。
- 提供零额外分配的遍历入口（回调式）。
