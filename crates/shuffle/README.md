# rstl-shuffle

`rstl-shuffle` 提供 `STL-style` 的 `Knuth/Fisher-Yates` 洗牌算法：

- `knuth_shuffle`
- `knuth_shuffle_with`
- `knuth_shuffle_range`
- `knuth_shuffle_range_with`
- `random_int`

本 crate 同时提供两种使用模式：

- 便捷默认：内置默认 RNG。
- 显式注入：通过 `*_with` 传入自定义 RNG（便于复现和测试）。
