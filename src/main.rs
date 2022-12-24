use core::num;
use std::time::Duration;
#[allow(unused)]
use std::{
    cell::{Cell, RefCell, UnsafeCell},
    collections::VecDeque,
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit},
    ops::{Deref, DerefMut},
    ptr::NonNull,
    rc::Rc,
    sync::{*, atomic::{*, Ordering::*}},
    thread::{self, Thread},
};

fn main() {
    let num_done = AtomicU64::new(0);
    let main_thread = thread::current();

    thread::scope(|s| {
        s.spawn(|| {
            for i in 0..100 {
                do_something();
                num_done.store(i + 1, Ordering::Relaxed);
                main_thread.unpark();
            }
        });
    });

    loop {
        let n = num_done.load(Ordering::Relaxed);
        println!("Completed: {}", n);
        if n == 100 {
            break;
        }
        thread::park();
    }

    println!("Done!")
}


fn do_something() {
    thread::sleep(Duration::from_millis(50))
}