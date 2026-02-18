# Thread Pool Implementation Notes

## Sources

- **"A Rust thread pool"** by Byron
    - URL: https://blog.batteson.com/2024/04/28/a-rust-thread-pool
    - Focus: Basic thread pool implementation using std library

- **"Multiple Thread Pools in Rust"** by Piotr Kołaczkowski
    - URL: https://pkolaczk.github.io/multiple-threadpools-rust/
    - Focus: Using Rayon for multiple custom thread pools

- **Rayon ThreadPool Documentation** (Official)
    - URL: https://docs.rs/rayon/latest/rayon/struct.ThreadPool.html
    - Focus: Complete Rayon ThreadPool API reference

- **"ThreadPool performance really bad"** - Rust User Forum
    - URL:
      https://users.rust-lang.org/t/threadpool-performance-really-bad/12485/18
    - Author: nathansizemore
    - Focus: Thread pool design for I/O workloads, async I/O architecture

---

## Basic Thread Pool Implementation

Based on Byron's blog post.

### Overview

The "wadingpool" implementation demonstrates how to build a simple thread pool
using only safe Rust and the standard library, serving as an educational example
of concurrent programming patterns in Rust.

### Context

- **Purpose**: Educational implementation to understand thread pool fundamentals
- **Constraints**: Uses only safe Rust and standard library (no external
  dependencies)
- **Use Case**: Compute-bound tasks (not ideal for IO-bound work as it blocks
  threads)
- **Name**: "wadingpool" - a metaphor for a shallow, educational implementation

### API Design Goals

The desired API interface:

```rust
let mut pool = ThreadPool::new(8);
let (tx, rx) = std::sync::mpsc::channel();
pool.spawn(move || {
    // do some computation
    let _ = tx.send(x);
});
```

Key design decisions:

- Leverage mpsc channel for task-to-main-thread communication
- Avoid complex generics and task handles
- Keep implementation simple and straightforward

### Core Implementation

**Data Structures**:

```rust
use std::thread::JoinHandle;
use std::sync::{self, mpsc::Sender, Arc, Mutex};

type Task = Box<dyn FnOnce() -> () + Send>;

pub struct ThreadPool {
    threads: Vec<JoinHandle<()>>,
    sender: Sender<Task>,
}
```

**Task Type**: `Box<dyn FnOnce() -> () + Send>`

- Uses boxed trait object for callbacks
- `FnOnce` ensures tasks can only be executed once
- `Send` allows tasks to be transferred between threads

**Thread Creation**:

- Uses `NonZeroUsize` to enforce thread count >= 1
- Pre-allocates thread vector capacity
- Each worker thread maintains a clone of the receiver

**Message Passing**:

- `mpsc::channel<Task>` for task distribution
- `Arc<Mutex<Receiver<Task>>>` for multi-consumer access
- Single producer, multiple consumers pattern

**Worker Thread Loop**:

```rust
let thread = std::thread::spawn(move || loop {
    let res = {
        let rx = receiver.lock().unwrap();
        rx.recv()
    };
    match res {
        Ok(task) => {
            task();
        }
        Err(_) => {
            return;
        }
    }
});
```

### Graceful Shutdown

**Implementation Changes**:

1. Update ThreadPool struct:

    ```rust
    sender: Option<Sender<Task>>,
    ```

2. Modified spawn method:
    - Check `sender` exists before sending
    - Panic if sender is missing (critical bug)

3. Drop Implementation:

    ```rust
    impl Drop for ThreadPool {
        fn drop(&mut self) {
            let mut sender = self.sender.take();
            drop(sender);

            while let Some(thread) = self.threads.pop() {
                thread.join().unwrap();
            }
        }
    }
    ```

**Shutdown Mechanism**:

- Dropping sender causes `recv()` to return `Err` on all worker threads
- Worker threads exit their loop when `recv()` fails
- Main thread waits for each worker to complete via `join()`
- Note: Cannot interrupt running tasks - shutdown may take time

### Potential Improvements

- Lock-free task queue for better short-task performance
- Task cancellation support
- Work-stealing for better load balancing
- Better error handling (replace panics with Results)
- Support for async tasks or futures
- Bounded vs unbounded task queue options
- Worker thread dynamic scaling

---

## Multiple Thread Pools with Rayon

Based on Piotr Kołaczkowski's blog post.

### Motivation and Use Cases

In complex programs that mix tasks of different types using different physical
resources (CPU, storage, network I/O), you may need to configure parallelism
levels differently for each task type. This is typically solved by scheduling
tasks of different types on dedicated thread pools.

**Real-world example**:

- Files on HDD: Multi-threaded access to a single HDD is a _really bad idea_ -
  use 1 thread per device
- Files on SSD: Can benefit from multiple threads (e.g., 16 threads)
- Both groups of files should be processed independently, in parallel

### Custom Thread Pools with Rayon

Based on both Piotr Kołaczkowski's blog post and official Rayon documentation.

Rayon allows building custom thread pools with `ThreadPoolBuilder`:

```rust
let pool = rayon::ThreadPoolBuilder::new()
    .num_threads(4)
    .build()
    .unwrap();
```

**Important**: `ThreadPool::new(configuration)` is deprecated in favor of
`ThreadPoolBuilder::build()`.

When the `ThreadPool` is dropped, the threads it manages will terminate after
completing any remaining work.

```rust
use std::thread;
let pool = rayon::ThreadPoolBuilder::new()
    .num_threads(4)
    .build()
    .unwrap();
pool.spawn(|| println!("Task executes on thread: {:?}", thread::current().id()));
pool.spawn(|| println!("Task executes on thread: {:?}", thread::current().id()));
```

**Channel-based approach** (replacing parallel iterators):

```rust
let pool = rayon::ThreadPoolBuilder::new()
    .num_threads(4)
    .build()
    .unwrap();
let files: Vec<std::path::PathBuf> = ...
let (tx, rx) = std::sync::mpsc::channel();
for f in files.into_iter() {
    let tx = tx.clone();
    pool.spawn(move || {
        tx.send(compute_hash(f)).unwrap();
    });
}
drop(tx); // need to close all senders, otherwise...
let hashes: Vec<FileHash> = rx.into_iter().collect();  // ... this would block
```

### Multiple Thread Pools

**Channel-based approach** (replacing parallel iterators):

```rust
let pool = rayon::ThreadPoolBuilder::new()
    .num_threads(4)
    .build()
    .unwrap();
let files: Vec<std::path::PathBuf> = ...
let (tx, rx) = std::sync::mpsc::channel();
for f in files.into_iter() {
    let tx = tx.clone();
    pool.spawn(move || {
        tx.send(compute_hash(f)).unwrap();
    });
}
drop(tx); // need to close all senders, otherwise...
let hashes: Vec<FileHash> = rx.into_iter().collect();  // ... this would block
```

Create separate pools for different resource types:

```rust
let hdd_pool = rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap();
let ssd_pool = rayon::ThreadPoolBuilder::new().num_threads(16).build().unwrap();

let files: Vec<std::path::PathBuf> = ...
let (tx, rx) = std::sync::mpsc::channel();
for f in files.into_iter() {
    let tx = tx.clone();
    let pool = if is_on_ssd(&f) {
        &ssd_pool
    } else {
        &hdd_pool
    };
    pool.spawn(move || {
        tx.send(compute_hash(f)).unwrap();
    });
}
drop(tx);
let hashes: Vec<FileHash> = rx.into_iter().collect();
```

### ThreadPool::spawn() and Lifetime

**Important**: `ThreadPool::spawn()` requires that the closure has a `'static`
lifetime because tasks run in the implicit, global scope and may outlive the
current stack frame. This means:

- Tasks cannot capture any references onto the stack
- You must use a `move` closure to transfer ownership
- Cannot borrow from local context (e.g., logger, configuration)

Example:

```rust
pool.spawn(move || {
    // This runs in global scope, may outlive current stack frame
    // Cannot borrow from outside, must own all data
});
```

**Spawn variants**:

- `spawn_fifo()`: Same as spawn but with FIFO ordering
- `spawn_broadcast()`: Spawns task on every thread in the pool (requires `Sync`)

### Scopes and Lifetime Issues

**Problem**: `ThreadPool.spawn()` method requires that the lambda has a
`'static` lifetime, which means it must not borrow any data other than global.
This makes it impossible to borrow from local context (e.g., logger,
configuration).

**Solution**: Use `pool.scope()` to create a scope that guarantees all tasks
finish before the scope exits. This allows tasks to borrow from the local
context:

```rust
let (tx, rx) = std::sync::mpsc::channel();
let logger: &Log = ...;
pool.scope(move |s| {
    for f in files.into_iter() {
        let tx = tx.clone();
        s.spawn(move |s| {
            logger.println(format!("Computing hash of: {}", f.display()));  // ok
            tx.send(f).unwrap();
        });
    }
});
```

### ThreadPool::install() - Executing Work in a Pool

`ThreadPool::install()` executes a closure within the thread pool. Any Rayon
operations (join, scope, parallel iterators) called inside will operate within
that thread pool.

```rust
fn main() {
    let pool = rayon::ThreadPoolBuilder::new().num_threads(8).build().unwrap();
    let n = pool.install(|| fib(20));
    println!("{}", n);
}

fn fib(n: usize) -> usize {
    if n == 0 || n == 1 {
        return n;
    }
    let (a, b) = rayon::join(|| fib(n - 1), || fib(n - 2)); // runs inside of `pool`
    return a + b;
}
```

**⚠️ Warning: thread-local data** Because the closure executes within the Rayon
thread pool, thread-local data from the current thread will not be accessible.

**⚠️ Warning: execution order** If the current thread is part of a different
thread pool, it will try to keep busy while the operation completes in its
target pool (similar to calling `yield_now()` in a loop). This may schedule
other tasks to run on the current thread in the meantime, affecting execution
order.

### ThreadPool::broadcast() - Executing on Every Thread

`ThreadPool::broadcast()` executes an operation on every thread in the thread
pool:

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

let pool = rayon::ThreadPoolBuilder::new().num_threads(5).build().unwrap();

// The argument gives context, including the index of each thread.
let v: Vec<usize> = pool.broadcast(|ctx| ctx.index() * ctx.index());
assert_eq!(v, &[0, 1, 4, 9, 16]);

// The closure can reference the local stack
let count = AtomicUsize::new(0);
pool.broadcast(|_| count.fetch_add(1, Ordering::Relaxed));
assert_eq!(count.into_inner(), 5);
```

**Execution strategy**: Broadcasts run after each thread exhausts its local work
queue, before attempting work-stealing. This runs everywhere timely without
being too disruptive to current work.

**⚠️ Warning**: Thread-local data from current thread not accessible.

**Panics**: If panic occurs on one or more threads, exactly one panic will be
propagated after all threads complete their operations.

### Multiple Nested Scopes

A single `Scope` in Rayon is always associated with a single `ThreadPool`. For
multiple thread pools, you need multiple scopes active at the same time. Scopes
can be nested:

```rust
let (tx, rx) = std::sync::mpsc::channel();
let logger: &Log = ...;
hdd_pool.scope(move |hdd_scope| {
    ssd_pool.scope(move |ssd_scope| {
        for f in files.into_iter() {
            let tx = tx.clone();
            if is_on_ssd(&f) {
              ssd_scope.spawn(move |s| { ... });
            } else {
              hdd_scope.spawn(move |s| { ... });
            }
        }
    });
});
```

### Dynamic Number of Thread Pools

**Problem**: The number of pools may not be known at compile time (e.g., one per
physical device). Nesting scopes manually becomes unwieldy.

**Solution**: Use a recursive `multi_scope` function:

```rust
fn nest<'scope, OP, R>(pools: &[&ThreadPool], scopes: Vec<&Scope<'scope>>, op: OP) -> R
where
    OP: FnOnce(&[&Scope<'scope>]) -> R + Send,
    R: Send,
{
    if !pools.is_empty() {
        pools[0].scope(move |s| {
            let mut scopes = scopes;
            scopes.push(s);
            nest(&pools[1..], scopes, op)
        })
    } else {
        (op)(&scopes)
    }
}

pub fn multi_scope<'scope, OP, R>(pools: &[&ThreadPool], op: OP) -> R
where
    OP: FnOnce(&[&Scope<'scope>]) -> R + Send,
    R: Send,
{
    nest(pools, Vec::with_capacity(pools.len()), op)
}
```

**Usage**:

```rust
let pools = [&hdd_pool, &ssd_pool]; // can be constructed dynamically
let common = vec![0, 1, 2]; // common data
multi_scope(&pools, |scopes| {
    scopes[0].spawn(|s| { /* execute on hdd_pool, can use &common */ });
    scopes[1].spawn(|s| { /* execute on ssd_pool, can use &common */ });
});
```

### Borrow Checker Challenges

**Historical issue (Rayon < 1.4.0)**:

- Rayon's `Scope` struct was _invariant_ over its lifetime parameter
- Cannot store references to scopes with different lifetimes in a single vector
- Required unsafe `transmute` to adjust lifetimes (not recommended)

**Current solution (Rayon >= 1.4.0)**:

- Rayon 1.4.0 changed the `scope` signature to allow `'scope` to be wider than
  the lambda lifetime
- This enables the safe recursive approach shown above without unsafe code
- When scopes are nested, they can all get the same `'scope` that holds the
  outermost lambda

### ThreadPool Scope Variants

Rayon provides multiple scope methods for different use cases:

- **`ThreadPool::scope()`**: Standard scope with LIFO ordering
- **`ThreadPool::scope_fifo()`**: Spawns from same thread prioritized in FIFO
  order
- **`ThreadPool::in_place_scope()`**: Creates a scope that spawns work into this
  pool (no Send requirement for the return value)
- **`ThreadPool::in_place_scope_fifo()`**: Same as in_place_scope with FIFO
  ordering

### ThreadPool Query Methods

**Thread information**:

- `current_num_threads()`: Returns current number of threads in the pool (may
  vary over time if not specified with ThreadPoolBuilder)
- `current_thread_index()`: Returns the index of the current thread if it's a
  Rayon worker in this pool, otherwise `None`
- `current_thread_has_pending_tasks()`: Returns true if current worker has local
  tasks pending (inherently racy due to work-stealing)

### ThreadPool Control Methods

**Join operations**:

```rust
pool.join(|| compute_a(), || compute_b()); // Equivalent to pool.install(|| join(...))
```

**Yielding**:

- `yield_now()`: Cooperatively yields execution to Rayon (returns `Yield` enum)
- `yield_local()`: Cooperatively yields execution to local Rayon work

Both methods return:

- `Some(Yield::Executed)` if anything was executed
- `Some(Yield::Idle)` if nothing was available
- `None` if current thread is not part of this pool

---

## Performance Considerations

### Task Granularity (from Byron's post)

**Testing Results**:

- Tested various task durations: few long tasks vs many short tasks
- Best performance observed: 100-500ms per task
- Minimum improvement across all tests: 6.4x speedup
- Tasks < 100ms show switching overhead costs

**Key Observations**:

- Tasks > 500ms hit parallelism limits (cannot divide work further)
- Tasks < 100ms incur context-switching overhead
- Lock contention may be a bottleneck for short tasks
- Suggests potential benefit from lock-free MPMC queues

**Trade-offs**:

- Mutex locking on receiver may impact short-lived tasks
- Lock-free primitives could improve performance for many small tasks
- Proper benchmarking needed before optimizing further

### Resource-Specific Parallelism (from Piotr's post)

**Different devices, different constraints**:

- CPU: Typically parallelizable across all cores
- HDD: Multi-threaded access to single HDD degrades performance - use 1 thread
  per device
- SSD: Can handle multiple threads effectively
- Network I/O: Often benefits from asynchronous rather than thread-based
  approaches

**Strategy**:

- Use dedicated thread pools for different resource types
- Configure thread counts based on device characteristics
- Process independent resource groups in parallel

---

## Thread Pool Design for I/O Workloads

Based on Rust User Forum discussion by nathansizemore.

### Async I/O vs Threading Philosophy

**Key insight**: The whole point of asynchronous I/O is that many things can be
done on one thread, because most of your operations are wait operations.

**Default thread architecture for I/O workloads**:

```rust
fn thread_one() {
    // 1. Wait for new connections
    // 2. Pass to event loop on success
}

fn thread_two() {
    // 1. Wait for event loop
    // 2. Read / Write data
    // 3. Handle I/O errors/disconnects
    // 4. Do stuff with data
    // 5. Re-arm socket
}
```

**Thread pool usage**:

- Use thread pools for CPU-bound work (step 4 above) - e.g., DB calls, heavy
  computation
- Don't use multiple threads for I/O waiting - that's what async I/O is for
- Only add more threads when actual implementation and measurements prove it's
  needed

### Performance Considerations

**Linux-specific issues**:

- Ethernet interrupts are assigned to one core by default
- Multiple cores triggered by interrupts specific to one core causes:
    - Context switching overhead
    - Cache misses
    - More performance cost than benefit

**Recommendation**: Start with minimal thread setup (1 listener + 1 I/O) and
only add complexity when necessary.

### Platform Considerations

**Testing environment**:

- Don't test on macOS with MIO if targeting Linux
- Linux uses epoll, macOS uses kqueue
- MIO's public API resembles epoll more than kqueue
- Testing on macOS won't reflect actual Linux performance characteristics

### Socket Configuration

**TcpListener**:

- Should be **blocking** (not edge or level triggered)
- Use backlog queue if MIO allows it

**Accepted sockets**:

- Should be **edge triggered** by default
- Edge triggered mode gives you control over performance

**Edge-triggered read pattern**:

```rust
fn read(client: &mut TcpStream) -> Result<Vec<u8>, ()> {
    let mut buf = Vec::<u8>::with_capacity(4098);
    let mut tmp_buf = [0u8; 4098];
    loop {
        let r_result = client.read(&mut tmp_buf);
        if r_result.is_err() {
            let e = r_result.unwrap_err();
            if e.kind() == ErrorKind::WouldBlock {
                return Ok(buf);
            } else {
                return Err(());
            }
        }

        let n_read = r_result.unwrap();
        let slice = &tmp_buf[0..n_read];
        buf.extend_from_slice(slice);
    }
}
```

**Key principle**: With edge-triggered sockets, you must read in a loop until
`WouldBlock` is returned to ensure all available data is consumed.

### Architecture Implications

**When NOT to use multiple threads**:

- Simple connection handling
- Basic I/O operations
- Waiting for network events

**When to use thread pools**:

- CPU-intensive data processing
- Database queries
- Computation-heavy operations
- Parallel processing of data

**Starting point**: Default to simple architecture and add complexity only when
measurements show it's needed.

---

## Key Takeaways

### Basic Thread Pool Concepts

1. **Simplicity**: Basic thread pool achievable with minimal code using std
   library
2. **Channels**: mpsc channels provide clean task distribution mechanism
3. **Shutdown**: Proper cleanup requires coordination via sender dropping
4. **Safety**: Rust's type system prevents common concurrency errors (memory
   safety)
5. **Task Granularity**: Consider task size - too small = overhead, too large =
   limited parallelism

### Multiple Thread Pool Concepts

1. **Resource-Specific Pools**: Different physical resources benefit from
   different parallelism levels
2. **Custom Pools**: Rayon's `ThreadPoolBuilder` enables pool-specific
   configuration
3. **Scopes**: Enable non-static closures by guaranteeing task completion before
   scope exit
4. **Dynamic Pools**: Recursive scope creation handles compile-time-unknown pool
   counts
5. **Rayon Evolution**: Newer versions simplify lifetime handling for multiple
   scopes

### I/O Workload Concepts

1. **Async I/O Philosophy**: One thread can handle many I/O operations because
   most time is spent waiting
2. **Minimal Architecture**: Start with 1 listener + 1 I/O thread, add
   complexity when needed
3. **Thread Pool Purpose**: Use for CPU-bound work (processing data), not I/O
   waiting
4. **Platform Awareness**: Test on target platform (epoll vs kqueue differences)
5. **Socket Configuration**: Blocking listener, edge-triggered accepted sockets
   with proper read loops
6. **Performance Trade-offs**: Multiple cores with single interrupt source cause
   context switching overhead

### Trade-offs and Limitations

- Simple implementation: Blocking IO, no task cancellation
- Multiple pools: Increased complexity, need careful pool sizing
- Scope-based: More complex than simple spawn, but enables borrowing
- Resource awareness: Requires understanding of device characteristics
- I/O workloads: Async I/O often better than threads for waiting operations

---

## Learning Resources

### Implementations

- **wadingpool** (basic std library implementation)
    - GitHub: https://github.com/battesonb/wadingpool
    - Example usage, tracing instrumentation, simulation code

- **fclones** (multi-pool implementation)
    - Demonstrates real-world use of multiple thread pools
    - Contains `multi_scope` implementation
