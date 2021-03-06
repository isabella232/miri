// ignore-windows: Concurrency on Windows is not supported yet.
// compile-flags: -Zmiri-disable-isolation

use std::thread::{spawn, sleep};
use std::ptr::null_mut;
use std::sync::atomic::{Ordering, AtomicPtr};
use std::time::Duration;

#[derive(Copy, Clone)]
struct EvilSend<T>(pub T);

unsafe impl<T> Send for EvilSend<T> {}
unsafe impl<T> Sync for EvilSend<T> {}

pub fn main() {
    // Shared atomic pointer
    let pointer = AtomicPtr::new(null_mut::<usize>());
    let ptr = EvilSend(&pointer as *const AtomicPtr<usize>);

    // Note: this is scheduler-dependent
    // the operations need to occur in
    // order, otherwise the allocation is
    // not visible to the other-thread to
    // detect the race:
    //  1. stack-allocate
    //  2. read
    //  3. stack-deallocate
    unsafe {
        let j1 = spawn(move || {
            // Concurrent allocate the memory.
            // Uses relaxed semantics to not generate
            // a release sequence.
            let pointer = &*ptr.0;

            let mut stack_var = 0usize;

            pointer.store(&mut stack_var as *mut _, Ordering::Release);
            
            sleep(Duration::from_millis(200));

            // Note: Implicit read for drop(_) races with write, would detect race with deallocate after.
            drop(stack_var); //~ ERROR Data race detected between Read on Thread(id = 1) and Write on Thread(id = 2)
        });

        let j2 = spawn(move || {
            let pointer = &*ptr.0;
            *pointer.load(Ordering::Acquire) = 3;
        });

        j1.join().unwrap();
        j2.join().unwrap();
    }
}
