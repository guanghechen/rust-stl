use core::cell::{Cell, RefCell};
use core::fmt;
use core::time::Duration;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::rc::Rc;
use std::sync::Arc;

use crate::Disposable;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservableError {
    DisposedUpdate,
    DisposedObserve,
    MissingScheduler,
}

impl fmt::Display for ObservableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DisposedUpdate => {
                write!(f, "[Observable] cannot update a disposed observable.")
            }
            Self::DisposedObserve => {
                write!(f, "[Ticker] cannot observe with a disposed ticker.")
            }
            Self::MissingScheduler => {
                write!(f, "[Observable] scheduler is required when delay > 0.")
            }
        }
    }
}

impl std::error::Error for ObservableError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservableNotifyError {
    ScheduleFailed,
    AsyncCallbackPanicked,
}

impl fmt::Display for ObservableNotifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ScheduleFailed => {
                write!(
                    f,
                    "[Observable] failed to schedule async notification task."
                )
            }
            Self::AsyncCallbackPanicked => {
                write!(f, "[Observable] async callback panicked.")
            }
        }
    }
}

impl std::error::Error for ObservableNotifyError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NextOptions {
    pub strict: bool,
    pub force: bool,
}

impl Default for NextOptions {
    fn default() -> Self {
        Self {
            strict: true,
            force: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubscribeOptions {
    pub replay: bool,
}

impl Default for SubscribeOptions {
    fn default() -> Self {
        Self { replay: true }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObserveOptions {
    pub strict: bool,
}

impl Default for ObserveOptions {
    fn default() -> Self {
        Self { strict: true }
    }
}

pub struct TickerOptions {
    pub start: u64,
    pub delay: Duration,
    pub scheduler: Option<Box<dyn Scheduler>>,
    pub on_error: Option<fn(ObservableNotifyError)>,
}

impl Default for TickerOptions {
    fn default() -> Self {
        Self {
            start: 0,
            delay: Duration::ZERO,
            scheduler: None,
            on_error: None,
        }
    }
}

pub struct ObservableOptions<T> {
    pub delay: Duration,
    pub equals: fn(&T, &T) -> bool,
    pub scheduler: Option<Box<dyn Scheduler>>,
    pub on_error: Option<fn(ObservableNotifyError)>,
}

impl<T> ObservableOptions<T> {
    pub fn new(equals: fn(&T, &T) -> bool) -> Self {
        Self {
            delay: Duration::ZERO,
            equals,
            scheduler: None,
            on_error: None,
        }
    }

    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    pub fn with_scheduler(mut self, scheduler: Box<dyn Scheduler>) -> Self {
        self.scheduler = Some(scheduler);
        self
    }

    pub fn with_on_error(mut self, on_error: fn(ObservableNotifyError)) -> Self {
        self.on_error = Some(on_error);
        self
    }
}

impl<T: PartialEq> Default for ObservableOptions<T> {
    fn default() -> Self {
        Self::new(default_equals::<T>)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ObservableChangeEvent<'a, T> {
    pub new: &'a T,
    pub old: Option<&'a T>,
    pub tick: u32,
}

#[derive(Clone)]
pub struct Subscription {
    active: Rc<Cell<bool>>,
}

pub struct UnobserveHandle {
    subscription: Option<Subscription>,
}

impl UnobserveHandle {
    fn new(subscription: Subscription) -> Self {
        Self {
            subscription: Some(subscription),
        }
    }

    fn noop() -> Self {
        Self { subscription: None }
    }

    pub fn unobserve(&mut self) {
        if let Some(subscription) = self.subscription.take() {
            subscription.unsubscribe();
        }
    }
}

impl Subscription {
    fn new_active() -> Self {
        Self {
            active: Rc::new(Cell::new(true)),
        }
    }

    fn noop() -> Self {
        Self {
            active: Rc::new(Cell::new(false)),
        }
    }

    pub fn unsubscribe(&self) {
        self.active.set(false);
    }

    pub fn is_active(&self) -> bool {
        self.active.get()
    }
}

type ObservableCallback<T> = dyn for<'a> FnMut(ObservableChangeEvent<'a, T>) + 'static;

struct SubscriberEntry<T> {
    active: Rc<Cell<bool>>,
    callback: Box<ObservableCallback<T>>,
}

struct PendingNotification<T> {
    old: Option<Arc<T>>,
    new: Option<Arc<T>>,
    tick: u32,
    timer_handle: Option<SchedulerHandle>,
}

impl<T> PendingNotification<T> {
    fn new() -> Self {
        Self {
            old: None,
            new: None,
            tick: 0,
            timer_handle: None,
        }
    }
}

pub trait ObservableLike<T>: Disposable {
    fn get_snapshot(&self) -> &T;
    fn get_tick(&self) -> u32;
    fn next(&mut self, value: T, options: NextOptions) -> Result<(), ObservableError>;
    fn subscribe<F>(&mut self, options: SubscribeOptions, callback: F) -> Subscription
    where
        F: for<'a> FnMut(ObservableChangeEvent<'a, T>) + 'static;
}

pub struct Ticker {
    observable: Rc<RefCell<Observable<u64>>>,
    observe_links: Vec<Subscription>,
}

impl Ticker {
    pub fn new(options: TickerOptions) -> Self {
        let ticker_options = ObservableOptions {
            delay: options.delay,
            equals: default_equals::<u64>,
            scheduler: options.scheduler,
            on_error: options.on_error,
        };

        Self {
            observable: Rc::new(RefCell::new(Observable::with_options(
                options.start,
                ticker_options,
            ))),
            observe_links: Vec::new(),
        }
    }

    pub fn get_snapshot(&self) -> u64 {
        *self.observable.borrow().get_snapshot()
    }

    pub fn get_tick(&self) -> u32 {
        self.observable.borrow().get_tick()
    }

    pub fn tick(&mut self, options: NextOptions) -> Result<(), ObservableError> {
        let current = self.get_snapshot();
        self.observable
            .borrow_mut()
            .next(current.wrapping_add(1), options)
    }

    pub fn subscribe<F>(&mut self, options: SubscribeOptions, callback: F) -> Subscription
    where
        F: for<'a> FnMut(ObservableChangeEvent<'a, u64>) + 'static,
    {
        self.observable.borrow_mut().subscribe(options, callback)
    }

    pub fn observe<T, O>(
        &mut self,
        observable: &mut O,
        options: ObserveOptions,
    ) -> Result<UnobserveHandle, ObservableError>
    where
        O: ObservableLike<T>,
    {
        let disposed = self.observable.borrow().is_disposed();
        if disposed {
            if options.strict {
                return Err(ObservableError::DisposedObserve);
            }
            return Ok(UnobserveHandle::noop());
        }

        if observable.is_disposed() {
            return Ok(UnobserveHandle::noop());
        }

        let ticker = Rc::clone(&self.observable);
        let subscription = observable.subscribe(SubscribeOptions { replay: true }, move |_| {
            let mut ticker = ticker.borrow_mut();
            let current = *ticker.get_snapshot();
            let _ = ticker.next(
                current.wrapping_add(1),
                NextOptions {
                    strict: false,
                    force: false,
                },
            );
        });

        self.observe_links.push(subscription.clone());
        Ok(UnobserveHandle::new(subscription))
    }
}

pub struct Observable<T> {
    value: Arc<T>,
    tick: u32,
    notify_tick: Rc<Cell<u32>>,
    is_notifying: Rc<Cell<bool>>,
    disposed: Rc<Cell<bool>>,
    delay: Duration,
    equals: fn(&T, &T) -> bool,
    scheduler: Option<Box<dyn Scheduler>>,
    on_error: Option<fn(ObservableNotifyError)>,
    subscribers: Rc<RefCell<Vec<SubscriberEntry<T>>>>,
    pending: Rc<RefCell<PendingNotification<T>>>,
}

impl<T: PartialEq + 'static> Observable<T> {
    pub fn new(default_value: T) -> Self {
        Self::with_options(default_value, ObservableOptions::default())
    }
}

impl<T: 'static> Observable<T> {
    pub fn with_options(default_value: T, options: ObservableOptions<T>) -> Self {
        Self {
            value: Arc::new(default_value),
            tick: 0,
            notify_tick: Rc::new(Cell::new(0)),
            is_notifying: Rc::new(Cell::new(false)),
            disposed: Rc::new(Cell::new(false)),
            delay: options.delay,
            equals: options.equals,
            scheduler: options.scheduler,
            on_error: options.on_error,
            subscribers: Rc::new(RefCell::new(Vec::new())),
            pending: Rc::new(RefCell::new(PendingNotification::new())),
        }
    }

    fn schedule_if_needed(&mut self) {
        if self.delay == Duration::ZERO {
            return;
        }

        let has_timer = self.pending.borrow().timer_handle.is_some();
        if has_timer {
            return;
        }

        let Some(scheduler) = self.scheduler.as_mut() else {
            return;
        };

        let subscribers = Rc::clone(&self.subscribers);
        let pending = Rc::clone(&self.pending);
        let notify_tick = Rc::clone(&self.notify_tick);
        let disposed = Rc::clone(&self.disposed);
        let on_error = self.on_error;

        let task = Box::new(move || {
            let result = catch_unwind(AssertUnwindSafe(|| {
                if disposed.get() {
                    return;
                }

                let (new, old, tick) = {
                    let mut pending = pending.borrow_mut();
                    pending.timer_handle = None;
                    let Some(new) = pending.new.take() else {
                        return;
                    };
                    let old = pending.old.take();
                    (new, old, pending.tick)
                };

                notify_subscribers(&subscribers, new.as_ref(), old.as_deref(), tick);
                notify_tick.set(tick);
            }));

            if let Err(payload) = result {
                if let Some(handler) = on_error {
                    handler(ObservableNotifyError::AsyncCallbackPanicked);
                } else {
                    resume_unwind(payload);
                }
            }
        });

        match scheduler.schedule_once(self.delay, task) {
            Ok(handle) => {
                self.pending.borrow_mut().timer_handle = Some(handle);
            }
            Err(error) => {
                self.handle_notify_error(error);
                self.flush_pending_internal(false);
            }
        }
    }

    fn cancel_pending_timer(&mut self) {
        let Some(scheduler) = self.scheduler.as_mut() else {
            return;
        };

        let handle = self.pending.borrow_mut().timer_handle.take();
        if let Some(handle) = handle {
            let _ = scheduler.cancel(handle);
        }
    }

    fn handle_notify_error(&self, error: ObservableNotifyError) {
        if let Some(handler) = self.on_error {
            handler(error);
        } else {
            panic!("{error}");
        }
    }

    fn flush_pending_internal(&mut self, allow_disposed: bool) {
        if self.is_notifying.get() {
            return;
        }

        self.is_notifying.set(true);
        while self.flush_pending_once_internal(allow_disposed) {}
        self.is_notifying.set(false);
    }

    fn flush_pending_once_internal(&mut self, allow_disposed: bool) -> bool {
        if self.disposed.get() && !allow_disposed {
            return false;
        }

        let (new, old, tick) = {
            let mut pending = self.pending.borrow_mut();
            let Some(new) = pending.new.take() else {
                return false;
            };
            pending.timer_handle = None;
            let old = pending.old.take();
            (new, old, pending.tick)
        };

        notify_subscribers(&self.subscribers, new.as_ref(), old.as_deref(), tick);
        self.notify_tick.set(tick);
        true
    }
}

impl<T: 'static> Disposable for Observable<T> {
    fn dispose(&mut self) {
        if self.disposed.get() {
            return;
        }

        self.disposed.set(true);
        self.cancel_pending_timer();

        if self.notify_tick.get() < self.tick {
            self.flush_pending_internal(true);
        }

        let mut subscribers = self.subscribers.borrow_mut();
        for entry in subscribers.iter() {
            entry.active.set(false);
        }
        subscribers.clear();
    }

    fn is_disposed(&self) -> bool {
        self.disposed.get()
    }
}

impl<T: 'static> ObservableLike<T> for Observable<T> {
    fn get_snapshot(&self) -> &T {
        self.value.as_ref()
    }

    fn get_tick(&self) -> u32 {
        self.tick
    }

    fn next(&mut self, value: T, options: NextOptions) -> Result<(), ObservableError> {
        if self.disposed.get() {
            if options.strict {
                return Err(ObservableError::DisposedUpdate);
            }
            return Ok(());
        }

        if self.delay > Duration::ZERO && self.scheduler.is_none() {
            return Err(ObservableError::MissingScheduler);
        }

        if !options.force && (self.equals)(&value, self.value.as_ref()) {
            return Ok(());
        }

        let old = core::mem::replace(&mut self.value, Arc::new(value));
        let new = Arc::clone(&self.value);
        self.tick = self.tick.wrapping_add(1);

        if self.delay == Duration::ZERO {
            notify_subscribers(
                &self.subscribers,
                new.as_ref(),
                Some(old.as_ref()),
                self.tick,
            );
            self.notify_tick.set(self.tick);
            return Ok(());
        }

        {
            let mut pending = self.pending.borrow_mut();
            if pending.old.is_none() {
                pending.old = Some(old);
            }
            pending.new = Some(new);
            pending.tick = self.tick;
        }
        self.schedule_if_needed();
        Ok(())
    }

    fn subscribe<F>(&mut self, options: SubscribeOptions, mut callback: F) -> Subscription
    where
        F: for<'a> FnMut(ObservableChangeEvent<'a, T>) + 'static,
    {
        if !self.disposed.get() && self.notify_tick.get() < self.tick {
            self.cancel_pending_timer();
            self.flush_pending_internal(false);
        }

        if options.replay {
            callback(ObservableChangeEvent {
                new: self.value.as_ref(),
                old: None,
                tick: self.tick,
            });
        }

        if self.disposed.get() {
            return Subscription::noop();
        }

        let subscription = Subscription::new_active();
        self.subscribers.borrow_mut().push(SubscriberEntry {
            active: subscription.active.clone(),
            callback: Box::new(callback),
        });
        subscription
    }
}

impl Disposable for Ticker {
    fn dispose(&mut self) {
        for subscription in &self.observe_links {
            subscription.unsubscribe();
        }
        self.observe_links.clear();
        self.observable.borrow_mut().dispose();
    }

    fn is_disposed(&self) -> bool {
        self.observable.borrow().is_disposed()
    }
}

fn notify_subscribers<T>(
    subscribers: &Rc<RefCell<Vec<SubscriberEntry<T>>>>,
    new: &T,
    old: Option<&T>,
    tick: u32,
) {
    let mut subscribers = subscribers.borrow_mut();
    for entry in subscribers.iter_mut() {
        if !entry.active.get() {
            continue;
        }
        (entry.callback)(ObservableChangeEvent { new, old, tick });
    }
    subscribers.retain(|entry| entry.active.get());
}

fn default_equals<T: PartialEq>(x: &T, y: &T) -> bool {
    x == y
}

#[cfg(test)]
mod tests {
    use super::{
        NextOptions, Observable, ObservableError, ObservableLike, ObservableNotifyError,
        ObservableOptions, ObserveOptions, Scheduler, SchedulerHandle, SubscribeOptions, Ticker,
        TickerOptions,
    };
    use crate::Disposable;
    use core::cell::RefCell;
    use core::time::Duration;
    use std::collections::{HashSet, VecDeque};
    use std::rc::Rc;

    thread_local! {
        static TEST_ERRORS: RefCell<Vec<ObservableNotifyError>> = const { RefCell::new(Vec::new()) };
    }

    fn reset_test_errors() {
        TEST_ERRORS.with(|errors| errors.borrow_mut().clear());
    }

    fn push_test_error(error: ObservableNotifyError) {
        TEST_ERRORS.with(|errors| errors.borrow_mut().push(error));
    }

    fn test_errors() -> Vec<ObservableNotifyError> {
        TEST_ERRORS.with(|errors| errors.borrow().clone())
    }

    #[derive(Default)]
    struct ManualSchedulerState {
        next_id: u64,
        fail_schedule: bool,
        cancelled: HashSet<SchedulerHandle>,
        tasks: VecDeque<(SchedulerHandle, Box<dyn FnOnce() + 'static>)>,
    }

    impl ManualSchedulerState {
        fn run_all(&mut self) {
            while let Some((handle, task)) = self.tasks.pop_front() {
                if self.cancelled.contains(&handle) {
                    continue;
                }
                task();
            }
        }
    }

    struct SharedManualScheduler {
        shared: Rc<RefCell<ManualSchedulerState>>,
    }

    impl SharedManualScheduler {
        fn new(shared: Rc<RefCell<ManualSchedulerState>>) -> Self {
            Self { shared }
        }
    }

    impl Scheduler for SharedManualScheduler {
        fn schedule_once(
            &mut self,
            _delay: Duration,
            task: Box<dyn FnOnce() + 'static>,
        ) -> Result<SchedulerHandle, ObservableNotifyError> {
            let mut shared = self.shared.borrow_mut();
            if shared.fail_schedule {
                return Err(ObservableNotifyError::ScheduleFailed);
            }

            shared.next_id = shared.next_id.wrapping_add(1);
            let handle = SchedulerHandle(shared.next_id);
            shared.tasks.push_back((handle, task));
            Ok(handle)
        }

        fn cancel(&mut self, handle: SchedulerHandle) -> bool {
            self.shared.borrow_mut().cancelled.insert(handle)
        }
    }

    #[test]
    fn snapshot_should_be_repeatable() {
        let observable = Observable::new(7i32);
        assert_eq!(*observable.get_snapshot(), 7);
        assert_eq!(*observable.get_snapshot(), 7);
    }

    #[test]
    fn subscribe_with_replay_should_emit_old_none() {
        let mut observable = Observable::new(10i32);
        let events: Rc<RefCell<Vec<(i32, Option<i32>, u32)>>> = Rc::new(RefCell::new(Vec::new()));
        let events_ref = Rc::clone(&events);

        observable.subscribe(SubscribeOptions::default(), move |event| {
            events_ref
                .borrow_mut()
                .push((*event.new, event.old.copied(), event.tick));
        });

        assert_eq!(events.borrow().as_slice(), &[(10, None, 0)]);
    }

    #[test]
    fn subscribe_without_replay_should_not_emit_immediately() {
        let mut observable = Observable::new(10i32);
        let called = Rc::new(RefCell::new(0usize));
        let called_ref = Rc::clone(&called);

        observable.subscribe(SubscribeOptions { replay: false }, move |_| {
            *called_ref.borrow_mut() += 1;
        });

        assert_eq!(*called.borrow(), 0);
    }

    #[test]
    fn next_should_emit_new_old_and_tick() {
        let mut observable = Observable::new(1i32);
        let events: Rc<RefCell<Vec<(i32, Option<i32>, u32)>>> = Rc::new(RefCell::new(Vec::new()));
        let events_ref = Rc::clone(&events);

        observable.subscribe(SubscribeOptions { replay: false }, move |event| {
            events_ref
                .borrow_mut()
                .push((*event.new, event.old.copied(), event.tick));
        });

        observable
            .next(2, NextOptions::default())
            .expect("next should succeed");
        observable
            .next(3, NextOptions::default())
            .expect("next should succeed");

        assert_eq!(
            events.borrow().as_slice(),
            &[(2, Some(1), 1), (3, Some(2), 2)]
        );
    }

    #[test]
    fn next_should_respect_equals_and_force() {
        let mut observable = Observable::new(9i32);
        let called = Rc::new(RefCell::new(0usize));
        let called_ref = Rc::clone(&called);
        observable.subscribe(SubscribeOptions { replay: false }, move |_| {
            *called_ref.borrow_mut() += 1;
        });

        observable
            .next(9, NextOptions::default())
            .expect("next should succeed");
        assert_eq!(*called.borrow(), 0);

        observable
            .next(
                9,
                NextOptions {
                    strict: true,
                    force: true,
                },
            )
            .expect("next should succeed");
        assert_eq!(*called.borrow(), 1);
        assert_eq!(observable.get_tick(), 1);
    }

    #[test]
    fn next_should_respect_disposed_strict_option() {
        let mut observable = Observable::new(1i32);
        observable.dispose();

        let result = observable.next(2, NextOptions::default());
        assert!(result.is_err());

        let result = observable.next(
            3,
            NextOptions {
                strict: false,
                force: false,
            },
        );
        assert!(result.is_ok());
        assert_eq!(*observable.get_snapshot(), 1);
    }

    #[test]
    fn next_should_return_missing_scheduler_when_delay_positive() {
        let mut observable = Observable::with_options(
            1i32,
            ObservableOptions::new(|x, y| x == y).with_delay(Duration::from_millis(5)),
        );

        let result = observable.next(2, NextOptions::default());
        assert!(matches!(result, Err(ObservableError::MissingScheduler)));
        assert_eq!(*observable.get_snapshot(), 1);
        assert_eq!(observable.get_tick(), 0);
    }

    #[test]
    fn notify_should_follow_fifo_order() {
        let mut observable = Observable::new(0i32);
        let order = Rc::new(RefCell::new(Vec::<u8>::new()));

        let order_ref1 = Rc::clone(&order);
        observable.subscribe(SubscribeOptions { replay: false }, move |_| {
            order_ref1.borrow_mut().push(1);
        });

        let order_ref2 = Rc::clone(&order);
        observable.subscribe(SubscribeOptions { replay: false }, move |_| {
            order_ref2.borrow_mut().push(2);
        });

        observable
            .next(1, NextOptions::default())
            .expect("next should succeed");
        assert_eq!(order.borrow().as_slice(), &[1, 2]);
    }

    #[test]
    fn unsubscribe_should_be_idempotent() {
        let mut observable = Observable::new(0i32);
        let called = Rc::new(RefCell::new(0usize));
        let called_ref = Rc::clone(&called);

        let sub = observable.subscribe(SubscribeOptions { replay: false }, move |_| {
            *called_ref.borrow_mut() += 1;
        });
        sub.unsubscribe();
        sub.unsubscribe();

        observable
            .next(1, NextOptions::default())
            .expect("next should succeed");
        assert_eq!(*called.borrow(), 0);
    }

    #[test]
    fn tick_should_wrap_on_overflow() {
        let mut observable = Observable::with_options(0u8, ObservableOptions::new(|x, y| x == y));

        observable.tick = u32::MAX;
        observable
            .next(1, NextOptions::default())
            .expect("next should succeed");

        assert_eq!(observable.get_tick(), 0);
    }

    #[test]
    fn delay_should_coalesce_to_last_value() {
        let scheduler_state = Rc::new(RefCell::new(ManualSchedulerState::default()));
        let scheduler = SharedManualScheduler::new(Rc::clone(&scheduler_state));

        let mut observable = Observable::with_options(
            0i32,
            ObservableOptions::new(|x, y| x == y)
                .with_delay(Duration::from_millis(10))
                .with_scheduler(Box::new(scheduler)),
        );

        let events = Rc::new(RefCell::new(Vec::<(i32, Option<i32>, u32)>::new()));
        let events_ref = Rc::clone(&events);
        observable.subscribe(SubscribeOptions { replay: false }, move |event| {
            events_ref
                .borrow_mut()
                .push((*event.new, event.old.copied(), event.tick));
        });

        observable
            .next(1, NextOptions::default())
            .expect("next should succeed");
        observable
            .next(2, NextOptions::default())
            .expect("next should succeed");
        observable
            .next(3, NextOptions::default())
            .expect("next should succeed");

        assert!(events.borrow().is_empty());

        scheduler_state.borrow_mut().run_all();

        assert_eq!(events.borrow().as_slice(), &[(3, Some(0), 3)]);
    }

    #[test]
    fn subscribe_should_flush_pending_before_replay() {
        let scheduler_state = Rc::new(RefCell::new(ManualSchedulerState::default()));
        let scheduler = SharedManualScheduler::new(Rc::clone(&scheduler_state));

        let mut observable = Observable::with_options(
            0i32,
            ObservableOptions::new(|x, y| x == y)
                .with_delay(Duration::from_millis(10))
                .with_scheduler(Box::new(scheduler)),
        );

        let existing_events = Rc::new(RefCell::new(Vec::<(i32, Option<i32>, u32)>::new()));
        let existing_events_ref = Rc::clone(&existing_events);
        observable.subscribe(SubscribeOptions { replay: false }, move |event| {
            existing_events_ref
                .borrow_mut()
                .push((*event.new, event.old.copied(), event.tick));
        });

        observable
            .next(1, NextOptions::default())
            .expect("next should succeed");

        let replay_events = Rc::new(RefCell::new(Vec::<(i32, Option<i32>, u32)>::new()));
        let replay_events_ref = Rc::clone(&replay_events);
        observable.subscribe(SubscribeOptions { replay: true }, move |event| {
            replay_events_ref
                .borrow_mut()
                .push((*event.new, event.old.copied(), event.tick));
        });

        assert_eq!(existing_events.borrow().as_slice(), &[(1, Some(0), 1)]);
        assert_eq!(replay_events.borrow().as_slice(), &[(1, None, 1)]);

        // Timer should be cancelled by subscribe-flush; running queued tasks should be no-op.
        scheduler_state.borrow_mut().run_all();

        assert_eq!(existing_events.borrow().as_slice(), &[(1, Some(0), 1)]);
        assert_eq!(replay_events.borrow().as_slice(), &[(1, None, 1)]);
    }

    #[test]
    fn dispose_should_flush_pending_notification() {
        let scheduler_state = Rc::new(RefCell::new(ManualSchedulerState::default()));
        let scheduler = SharedManualScheduler::new(Rc::clone(&scheduler_state));

        let mut observable = Observable::with_options(
            0i32,
            ObservableOptions::new(|x, y| x == y)
                .with_delay(Duration::from_millis(10))
                .with_scheduler(Box::new(scheduler)),
        );

        let events = Rc::new(RefCell::new(Vec::<(i32, Option<i32>, u32)>::new()));
        let events_ref = Rc::clone(&events);
        observable.subscribe(SubscribeOptions { replay: false }, move |event| {
            events_ref
                .borrow_mut()
                .push((*event.new, event.old.copied(), event.tick));
        });

        observable
            .next(1, NextOptions::default())
            .expect("next should succeed");
        observable
            .next(2, NextOptions::default())
            .expect("next should succeed");

        observable.dispose();

        assert_eq!(events.borrow().as_slice(), &[(2, Some(0), 2)]);

        scheduler_state.borrow_mut().run_all();
        assert_eq!(events.borrow().as_slice(), &[(2, Some(0), 2)]);
    }

    #[test]
    fn on_error_should_receive_schedule_failed() {
        reset_test_errors();

        let scheduler_state = Rc::new(RefCell::new(ManualSchedulerState {
            fail_schedule: true,
            ..Default::default()
        }));
        let scheduler = SharedManualScheduler::new(Rc::clone(&scheduler_state));

        let mut observable = Observable::with_options(
            0i32,
            ObservableOptions::new(|x, y| x == y)
                .with_delay(Duration::from_millis(10))
                .with_scheduler(Box::new(scheduler))
                .with_on_error(push_test_error),
        );

        let events = Rc::new(RefCell::new(Vec::<i32>::new()));
        let events_ref = Rc::clone(&events);
        observable.subscribe(SubscribeOptions { replay: false }, move |event| {
            events_ref.borrow_mut().push(*event.new);
        });

        observable
            .next(1, NextOptions::default())
            .expect("next should succeed");

        assert_eq!(
            test_errors().as_slice(),
            &[ObservableNotifyError::ScheduleFailed]
        );
        assert_eq!(events.borrow().as_slice(), &[1]);
    }

    #[test]
    fn on_error_should_receive_async_callback_panicked() {
        reset_test_errors();

        let scheduler_state = Rc::new(RefCell::new(ManualSchedulerState::default()));
        let scheduler = SharedManualScheduler::new(Rc::clone(&scheduler_state));

        let mut observable = Observable::with_options(
            0i32,
            ObservableOptions::new(|x, y| x == y)
                .with_delay(Duration::from_millis(10))
                .with_scheduler(Box::new(scheduler))
                .with_on_error(push_test_error),
        );

        observable.subscribe(SubscribeOptions { replay: false }, |_| {
            panic!("boom");
        });

        observable
            .next(1, NextOptions::default())
            .expect("next should succeed");

        scheduler_state.borrow_mut().run_all();

        assert_eq!(
            test_errors().as_slice(),
            &[ObservableNotifyError::AsyncCallbackPanicked]
        );
    }

    #[test]
    #[should_panic(expected = "boom")]
    fn sync_callback_panic_should_propagate() {
        let mut observable = Observable::new(0i32);
        observable.subscribe(SubscribeOptions { replay: false }, |_| {
            panic!("boom");
        });

        let _ = observable.next(1, NextOptions::default());
    }

    #[test]
    fn ticker_tick_should_increment_snapshot() {
        let mut ticker = Ticker::new(TickerOptions::default());
        assert_eq!(ticker.get_snapshot(), 0);

        ticker
            .tick(NextOptions::default())
            .expect("tick should succeed");
        ticker
            .tick(NextOptions::default())
            .expect("tick should succeed");

        assert_eq!(ticker.get_snapshot(), 2);
        assert_eq!(ticker.get_tick(), 2);
    }

    #[test]
    fn ticker_observe_should_tick_on_replay_and_updates() {
        let mut ticker = Ticker::new(TickerOptions::default());
        let mut source = Observable::new(10i32);

        let mut handle = ticker
            .observe(&mut source, ObserveOptions::default())
            .expect("observe should succeed");

        assert_eq!(ticker.get_snapshot(), 1);

        source
            .next(11, NextOptions::default())
            .expect("next should succeed");
        assert_eq!(ticker.get_snapshot(), 2);

        handle.unobserve();
        handle.unobserve();

        source
            .next(12, NextOptions::default())
            .expect("next should succeed");
        assert_eq!(ticker.get_snapshot(), 2);
    }

    #[test]
    fn ticker_observe_should_handle_disposed_strict_option() {
        let mut ticker = Ticker::new(TickerOptions::default());
        let mut source = Observable::new(1i32);
        ticker.dispose();

        let result = ticker.observe(&mut source, ObserveOptions::default());
        assert!(matches!(result, Err(ObservableError::DisposedObserve)));

        let result = ticker.observe(&mut source, ObserveOptions { strict: false });
        assert!(result.is_ok());
    }

    #[test]
    fn ticker_dispose_should_unobserve_all_links() {
        let mut ticker = Ticker::new(TickerOptions::default());
        let mut source = Observable::new(1i32);

        let _handle = ticker
            .observe(&mut source, ObserveOptions::default())
            .expect("observe should succeed");
        assert_eq!(ticker.get_snapshot(), 1);

        ticker.dispose();
        assert!(ticker.is_disposed());

        source
            .next(2, NextOptions::default())
            .expect("next should succeed");
        assert_eq!(ticker.get_snapshot(), 1);
    }
}
