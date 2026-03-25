# Observable 最终设计（`rstl-collection`）

状态：`Final`

## 1. 目标

`Observable` 提供两类核心能力：

- 可重复读取的 `snapshot`。
- 当 `value` 发生有效变化时，向订阅者发送变更通知。

本设计对齐 `sora/packages/observable` 的核心语义，并按 `rust-stl` 约束收敛为可直接实现的规范。

## 2. 范围

包含：

- `Observable<T>`：`snapshot`、`tick`、`subscribe`、`next`、`dispose`。
- `Ticker`：`Observable<u64>` 的特化，支持 `observe`。
- `delay` trailing debounce（通过 `Scheduler` 实现，不绑定特定 runtime）。

不包含：

- FRP 高阶算子（`map/filter/combineLatest` 等）。
- 业务层 notification 系统实现。
- 首版线程安全容器（先交付单线程 `&mut self` 语义）。

## 3. 模块边界（SRP）

建议实现目录：`crates/collection/src/observable/`

- `core.rs`：`Observable<T>` 状态机与语义。
- `subscriber.rs`：订阅者注册表、`Subscription`、幂等取消。
- `scheduler.rs`：`Scheduler` trait 与默认实现。
- `ticker.rs`：`Ticker` 与 `observe/unobserve`。
- `error.rs`：错误类型。

依赖方向：

```text
ticker  -> core -> subscriber
core    -> scheduler
core    -> error
```

## 4. API 规范

```rust
use core::time::Duration;

pub struct ObservableOptions<T> {
    pub delay: Duration,
    pub equals: fn(&T, &T) -> bool,
    pub scheduler: Option<Box<dyn Scheduler>>,
    pub on_error: Option<fn(ObservableNotifyError)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SchedulerHandle(pub u64);

pub trait Scheduler {
    fn schedule_once(
        &mut self,
        delay: Duration,
        task: Box<dyn FnOnce() + 'static>,
    ) -> Result<SchedulerHandle, ObservableNotifyError>;

    fn cancel(&mut self, handle: SchedulerHandle) -> bool;
}

pub struct NextOptions {
    pub strict: bool, // default: true
    pub force: bool,  // default: false
}

pub struct SubscribeOptions {
    pub replay: bool, // default: true
}

pub struct ObserveOptions {
    pub strict: bool, // default: true
}

pub struct ObservableChangeEvent<'a, T> {
    pub new: &'a T,
    pub old: Option<&'a T>,
    pub tick: u32,
}

pub trait ObservableLike<T>: Disposable {
    fn get_snapshot(&self) -> &T;
    fn get_tick(&self) -> u32;
    fn next(&mut self, value: T, options: NextOptions) -> Result<(), ObservableError>;
    fn subscribe<F>(&mut self, options: SubscribeOptions, callback: F) -> Subscription
    where
        F: FnMut(ObservableChangeEvent<'_, T>) + 'static;
}

pub struct Observable<T> { /* omitted */ }

pub struct Ticker { /* Observable<u64> wrapper */ }

impl Ticker {
    pub fn tick(&mut self, options: NextOptions) -> Result<(), ObservableError>;

    pub fn observe<T, O>(
        &mut self,
        observable: &mut O,
        options: ObserveOptions,
    ) -> Result<UnobserveHandle, ObservableError>
    where
        O: ObservableLike<T>;
}
```

默认约束：

- `equals` 默认使用 `PartialEq`（若 `T: PartialEq`），或由调用方显式提供。
- `Subscription`、`UnobserveHandle` 必须幂等释放。
- `tick` 每次有效更新执行 `wrapping_add(1)`。
- `Scheduler::cancel` 必须幂等。
- 对已取消 handle，scheduler 必须保证对应 task 不会再次执行。

## 5. 语义规范

### 5.1 `snapshot` 与 `tick`

- `get_snapshot()` 复杂度 `O(1)`，可重复读取。
- 在两次有效 `next` 之间，多次 `get_snapshot()` 返回一致。
- `tick` 是更新计数，不是通知计数。

### 5.2 `subscribe`

- `subscribe(options, callback)` 注册订阅。
- `replay=true` 时，订阅建立后立即收到一次当前 `snapshot` 通知。
- `replay` 事件固定 `old=None`，`new=current_snapshot`，`tick=current_tick`。
- `replay=false` 时，只有后续有效更新才通知。
- 若 observable 已 disposed：
  - `replay=true` 仍回放一次当前值。
  - 返回 noop subscription（立即失效）。
- 复杂度：
  - 注册 `O(1)`。
  - 若注册前触发内部 `flush`，附加 `O(n)`（`n` 为有效订阅者数）。

### 5.3 `next`

- 默认 `strict=true`。
- disposed 后：
  - `strict=true` 返回 `DisposedUpdate`。
  - `strict=false` 静默 no-op。
- 去重规则：
  - `force=false` 且 `equals(new, current)==true` 时 no-op。
  - 否则更新 `value`，并执行 `tick = tick.wrapping_add(1)`。
- 通知载荷固定为：`new`、`old`、`tick`。
- 复杂度：
  - `delay=0`：`O(n)`。
  - `delay>0`：调度路径 `O(1)`，实际通知 `O(n)`。
- 重入（reentrancy）语义：
  - 允许在订阅回调内再次调用 `next`。
  - 同一轮通知期间产生的新更新不做递归展开，进入下一轮 `flush` 处理。
  - 实现上要求使用 queue/loop 驱动，避免递归通知导致栈增长失控。

### 5.4 通知顺序与取消时机

- 通知顺序固定为订阅注册顺序（FIFO）。
- 每一轮通知开始时，对有效订阅者列表做快照。
- 在回调中执行 `unsubscribe`：
  - 不影响当前轮已经快照出的遍历顺序。
  - 从下一轮通知开始生效。
- 在回调中新增订阅：
  - 不参与当前轮通知。
  - 从下一轮通知开始生效（若 `replay=true`，仍会立即收到 replay）。

### 5.5 debounce（`delay > 0`）

- 语义：trailing debounce。
- 同一时刻最多一个 active timer。
- 连续多次更新仅保证最后一次值被通知。
- `tick` 代表更新计数，因此在 debounce 合并后，通知中的 `tick` 可能跳号（预期行为）。
- 实现策略对齐 TS：
  - 首次 pending 更新触发 `schedule_once`。
  - 回调执行后若仍有 pending 更新，再次 `schedule_once`（循环推进）。
  - `dispose` 或 `flush` 时调用 `cancel` 清理 active timer。
- 异步通知失败时调用 `on_error`；未配置时按默认错误策略处理。

### 5.6 同步回调错误策略

- 同步通知路径中，订阅回调发生 `panic` 时直接向上传播。
- 该 `panic` 会中断当前通知轮，不做吞掉异常后继续通知。
- `on_error` 仅用于异步调度/通知路径（例如 scheduler 回调）。

### 5.7 `dispose`

- 幂等：可重复调用且无额外副作用。
- 顺序：
  - 先设置 `disposed=true`。
  - 取消 pending timer。
  - 若存在 `update_tick > notify_tick`，执行一次 internal `flush_pending()`。
    - `flush_pending()` 为 private/internal 路径，绕过 public `next` 的 disposed 检查。
  - 释放全部订阅。
- 后置条件：
  - `is_disposed() == true`。
  - 后续 `subscribe` / `next` 遵循 disposed 语义。

### 5.8 `Ticker.observe`

- 订阅目标 observable，每次收到目标通知即 `tick()`。
- 与参考实现保持一致：`observe` 建立时若目标 `replay` 生效，会导致 ticker 立即增加一次。
- disposed ticker：
  - `strict=true` 返回 `DisposedObserve`。
  - `strict=false` 返回 noop `UnobserveHandle`。

### 5.9 `u32` 自然溢出

- `tick` 使用 `u32`，并通过 `wrapping_add` 明确自然溢出。
- 该行为在不同构建模式下保持一致。
- `tick` 不可用于长时间跨度全序比较或全局唯一去重。

## 6. 错误模型

```rust
pub enum ObservableError {
    DisposedUpdate,
    DisposedObserve,
    MissingScheduler,
}

pub enum ObservableNotifyError {
    ScheduleFailed,
    AsyncCallbackPanicked,
}
```

触发条件：

- `DisposedUpdate`：disposed observable 上 `next(..., strict=true)`。
- `DisposedObserve`：disposed ticker 上 `observe(..., strict=true)`。
- `MissingScheduler`：`delay > 0` 且无可用 scheduler。
- `ScheduleFailed`：scheduler 无法成功创建定时任务。
- `AsyncCallbackPanicked`：异步回调执行触发 panic。

## 7. 测试要求

必须覆盖：

- `snapshot` 可重复读取。
- `subscribe(replay=true)` 立即回放。
- `subscribe(replay=true)` 的事件字段固定为 `old=None`。
- `subscribe(replay=false)` 不回放。
- `equals` 去重与 `force` 强制通知。
- 通知顺序 FIFO。
- 回调内 `unsubscribe` 对下一轮生效。
- 回调内 `next` 的 reentrancy 进入下一轮 `flush`，不递归展开。
- disposed 下 `strict=true/false` 分支。
- `delay` trailing debounce 与 tick 跳号语义。
- 同步回调 `panic` 向上传播并中断当前轮。
- `dispose` 前 pending 更新的 flush。
- `ticker.observe/unobserve` 行为与幂等。
- `tick` 在 `u32::MAX` 后自然溢出。

## 8. benchmark 要求

- `next` 吞吐：`n=1/10/100` 订阅者，`delay=0`。
- 高频更新合并效果：`delay>0`。
- 订阅 churn 成本：`subscribe/unsubscribe` 高频场景。

## 9. 分阶段实现

1. Phase 1：`Observable<T>` 同步路径（`delay=0`）。
2. Phase 2：`Scheduler` 与 `delay` debounce。
3. Phase 3：`Ticker` 与 `observe/unobserve`。
4. Phase 4：benchmark 与文档复杂度标注。
