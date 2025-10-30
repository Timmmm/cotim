// Async/await runtime for Rust verification of SystemVerilog designs.

// This doesn't use `waker()` - instead the task is polled on every cycle.
// There is only one task.

use std::{future::Future, pin::Pin, sync::{mpsc::{sync_channel, Receiver, SyncSender}, Arc, Mutex}, task::Context};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Task executor that receives tasks off of a channel and runs them.
struct Executor {
    ready_queue: Receiver<Arc<Task>>,
}

/// `Spawner` spawns new futures onto the task channel.
#[derive(Clone)]
struct Spawner {
    task_sender: SyncSender<Arc<Task>>,
}

/// A future that can reschedule itself to be polled by an `Executor`.
struct Task {
    /// In-progress future that should be pushed to completion.
    ///
    /// The `Mutex` is not necessary for correctness, since we only have
    /// one thread executing tasks at once. However, Rust isn't smart
    /// enough to know that `future` is only mutated from one thread,
    /// so we need to use the `Mutex` to prove thread-safety. A production
    /// executor would not need this, and could use `UnsafeCell` instead.
    future: Mutex<Option<BoxFuture<'static, ()>>>,

    /// Handle to place the task itself back onto the task queue.
    task_sender: SyncSender<Arc<Task>>,
}

fn new_executor_and_spawner() -> (Executor, Spawner) {
    // Maximum number of tasks to allow queueing in the channel at once.
    // This is just to make `sync_channel` happy, and wouldn't be present in
    // a real executor.
    const MAX_QUEUED_TASKS: usize = 10_000;
    let (task_sender, ready_queue) = sync_channel(MAX_QUEUED_TASKS);
    (Executor { ready_queue }, Spawner { task_sender })
}

impl Spawner {
    fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        let future = future.boxed();
        let task = Arc::new(Task {
            future: Mutex::new(Some(future)),
            task_sender: self.task_sender.clone(),
        });
        self.task_sender.try_send(task).expect("too many tasks queued");
    }
}

pub trait ArcWake: Send + Sync {
    fn wake(self: Arc<Self>) {
        Self::wake_by_ref(&self)
    }

    fn wake_by_ref(arc_self: &Arc<Self>);
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        // Implement `wake` by sending this task back onto the task channel
        // so that it will be polled again by the executor.
        let cloned = arc_self.clone();
        arc_self
            .task_sender
            .try_send(cloned)
            .expect("too many tasks queued");
    }
}

impl Executor {
    fn run(&self) {
        while let Ok(task) = self.ready_queue.recv() {
            // Take the future, and if it has not yet completed (is still Some),
            // poll it in an attempt to complete it.
            let mut future_slot = task.future.lock().unwrap();
            if let Some(mut future) = future_slot.take() {
                // Create a `LocalWaker` from the task itself
                let waker = waker_ref(&task);
                let context = &mut Context::from_waker(&waker);
                // `BoxFuture<T>` is a type alias for
                // `Pin<Box<dyn Future<Output = T> + Send + 'static>>`.
                // We can get a `Pin<&mut dyn Future + Send + 'static>`
                // from it by calling the `Pin::as_mut` method.
                if future.as_mut().poll(context).is_pending() {
                    // We're not done processing the future, so put it
                    // back in its task to be run again in the future.
                    *future_slot = Some(future);
                }
            }
        }
    }
}

// Stimulus code will be like this:

// async fn cpu_test() {
//     loop {
//         select!(
//             stimulus(),
//             checkers(),
//         );
//     }
// }

// async fn stimulus() {
//     loop {
//         select!(
//             memory_agent(),

//         )
//     }
// }

// async fn load_elf() {

// }

// async fn inject_interrupt() {

// }

// async fn memory_agent() {
//     loop {
//         select!(
//             memory_recv(),
//             memory_send(),
//         )
//     }
// }

async fn send_transaction(clock: &Clock, ready: &Wire, valid: &mut Wire, bus: &[&mut Wire], data: &str) {
    ready.high().await;
    valid.set_high();
    bus[0].set_low(); // ...
    clock.cycle().await;
    valid.set_low();
}


async fn recv_transaction(clock: &Clock, ready: &Wire, bus: &[&mut Wire]) -> String {
    // ...
    todo!()
}

async fn spi_test(/* all the wires */) {
    send_transaction(clock, ready, valid, bus, data).await;
    recv_transaction(clock, ready, bus).await;
    // TODO: How do we send the received/sent transactions to the model?
    // Probably just do the model checking totally separately?
}

// TODO: We want reusable stimulus across environments, so that your random memory transaction generator (or whatever)
// can be used in both the memory block testbench and the top level one.

// TODO: What we *really* want is to export the entire SV structure at compile
// time to Rust so if you change something in the SV it's accessible at compile time in Rust.
// That's a bit of a catch 22 though.

// So the SV code is like:

fn init() {
    self.task = async_top(&self.wire_clock, &mut self.wire_ready);
}

fn tick(inputs: ...) {
    // Set the values of all the wires.
    self.wire_clock.set(inputs.clock);
    self.wire_ready.set(inputs.ready);

    // Poll the future. We don't need to worry about wakers (for wires) because we poll every cycle.
    // We only

    while self.task.poll() != Waiting {}
}
