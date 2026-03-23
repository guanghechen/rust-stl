# rstl-manacher

`rstl-manacher` 提供 `STL-style` 的 Manacher 回文半径算法：

- `manacher<T: Eq>(&[T]) -> Vec<usize>`
- `manacher_by(len, eq) -> Vec<usize>`（index 访问模型）
- `manacher_str(&str) -> Vec<usize>`（按 UTF-8 bytes 计算）

返回的 `radius` 长度为 `2n - 1`：

- `radius[2 * i]` 对应中心 `(i, i)` 的最长回文半径。
- `radius[2 * i + 1]` 对应中心 `(i, i + 1)` 的最长回文半径。

复杂度：

- 时间：`O(n)`
- 额外空间：`O(n)`
