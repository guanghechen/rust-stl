use std::cell::RefCell;
use std::collections::{HashSet, VecDeque};
use std::rc::Rc;
use std::time::Duration;

use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use rstl_collection::{
    NextOptions, Observable, ObservableLike, ObservableNotifyError, ObservableOptions, Scheduler,
    SchedulerHandle, SubscribeOptions,
};

const NEXT_N: usize = 20_000;
const CHURN_N: usize = 10_000;
const COALESCE_N: usize = 10_000;

#[derive(Default)]
struct ManualSchedulerState {
    next_id: u64,
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
        shared.next_id = shared.next_id.wrapping_add(1);
        let handle = SchedulerHandle(shared.next_id);
        shared.tasks.push_back((handle, task));
        Ok(handle)
    }

    fn cancel(&mut self, handle: SchedulerHandle) -> bool {
        self.shared.borrow_mut().cancelled.insert(handle)
    }
}

fn bench_next_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("collection/observable_next_throughput");

    for &subscriber_count in &[1usize, 10, 100] {
        group.bench_function(format!("subscribers_{subscriber_count}"), |b| {
            b.iter_batched(
                || {
                    let mut observable = Observable::new(0u64);
                    for _ in 0..subscriber_count {
                        observable.subscribe(SubscribeOptions { replay: false }, |_| {});
                    }
                    observable
                },
                |mut observable| {
                    for i in 1..=NEXT_N {
                        observable
                            .next(i as u64, NextOptions::default())
                            .expect("next should succeed");
                    }
                    black_box(observable.get_tick());
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

fn bench_delay_coalesce(c: &mut Criterion) {
    let mut group = c.benchmark_group("collection/observable_delay_coalesce");

    group.bench_function("delay_with_manual_scheduler", |b| {
        b.iter_batched(
            || {
                let scheduler_state = Rc::new(RefCell::new(ManualSchedulerState::default()));
                let scheduler = SharedManualScheduler::new(Rc::clone(&scheduler_state));

                let mut observable = Observable::with_options(
                    0u64,
                    ObservableOptions::new(|x, y| x == y)
                        .with_delay(Duration::from_millis(10))
                        .with_scheduler(Box::new(scheduler)),
                );

                let notify_counter = Rc::new(RefCell::new(0usize));
                let notify_counter_ref = Rc::clone(&notify_counter);
                observable.subscribe(SubscribeOptions { replay: false }, move |_| {
                    *notify_counter_ref.borrow_mut() += 1;
                });

                (observable, scheduler_state, notify_counter)
            },
            |(mut observable, scheduler_state, notify_counter)| {
                for i in 1..=COALESCE_N {
                    observable
                        .next(i as u64, NextOptions::default())
                        .expect("next should succeed");
                }

                scheduler_state.borrow_mut().run_all();

                black_box(observable.get_tick());
                black_box(*notify_counter.borrow());
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_subscribe_churn(c: &mut Criterion) {
    let mut group = c.benchmark_group("collection/observable_subscribe_churn");

    group.bench_function("subscribe_unsubscribe_10k", |b| {
        b.iter_batched(
            || Observable::new(0u64),
            |mut observable| {
                for _ in 0..CHURN_N {
                    let subscription =
                        observable.subscribe(SubscribeOptions { replay: false }, |_| {});
                    subscription.unsubscribe();
                }

                observable
                    .next(1, NextOptions::default())
                    .expect("next should succeed");

                black_box(observable.get_tick());
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(
    observable_common,
    bench_next_throughput,
    bench_delay_coalesce,
    bench_subscribe_churn
);
criterion_main!(observable_common);
