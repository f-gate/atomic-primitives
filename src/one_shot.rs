
use core::panic;
use std::{mem::MaybeUninit, sync::atomic::{AtomicBool, Ordering::*}, cell::UnsafeCell, fmt::write};


unsafe impl<T: Send> Send for OneShotChannel<T> {}

struct OneShotChannel<T> {
    is_ready: AtomicBool,
    in_use: AtomicBool,
    message: UnsafeCell<MaybeUninit<T>>,
}

impl<T> OneShotChannel<T> {
    fn new() -> Self {
        Self {
            is_ready: AtomicBool::new(false),
            in_use: AtomicBool::new(false),
            message: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    fn send(&self, message: T) {
        if self.in_use.swap(true, Relaxed) {
            panic!("Already in use!");
        }
            unsafe {
                (*self.message.get()).write(message);
                self.is_ready.store(true, Release);
            }
    }

    fn recieve(&self) -> T {
        if !self.is_ready.swap(false, Acquire) {
            panic!("Message is not ready yet.")
        }
        // We have checked and reset the ready flag so is safe.
        unsafe {
                (*self.message.get()).assume_init_read()
            }
    }

    /// Acquire load is_ready.
    fn is_ready(&self) -> bool {
        self.is_ready.load(Relaxed)
    }
}

impl<T> Drop for OneShotChannel<T> {
    fn drop(&mut self) {
        // if we can get a mutable reference here then we have exclusive ownership.
        if *self.is_ready.get_mut() {
            unsafe {
                // Drop the message to avoid leaks/
                self.message.get_mut().assume_init_drop()
            }
        }
    }
}