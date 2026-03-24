# rstl-trie

`rstl-trie` 提供泛型序列 Trie（prefix tree）实现，支持任意元素类型（不仅限字符串）：

- 文本字符序列
- keystroke / 组合键序列
- 自定义 token 序列

核心特性：

- 自定义字母表大小：`sigma_size`
- 自定义索引映射：`idx(&E) -> usize`
- 重复插入同一 key 的值合并策略：`merge_node_value(prev, next)`
- `slice-first` API：`try_* + Result`

示例：

```rust
use rstl_trie::{Trie, TrieOptions, alpha_numeric_idx};

let mut trie = Trie::new(TrieOptions {
    sigma_size: 62,
    idx: alpha_numeric_idx,
    merge_node_value: |x, y| x + y,
})?;

let key: Vec<char> = "cat".chars().collect();
trie.try_insert(&key, 1)?;
trie.try_insert(&key, 2)?;
assert_eq!(trie.try_get(&key)?, Some(&3));
```
