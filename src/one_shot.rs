
use core::panic;
use std::{mem::MaybeUninit, sync::atomic::{AtomicBool, Ordering::*}, cell::UnsafeCell, fmt::write, thread::Thread};


unsafe impl<T: Send> Send for OneShotChannel<T> {}

struct OneShotChannel<T> {
    is_ready: AtomicBool,
    in_use: AtomicBool,
    message: UnsafeCell<MaybeUninit<T>>,
}

struct Sender<'a, T> {
    channel: &'a OneShotChannel<T>,
}

struct Reciever<'a, T> {
    channel: &'a OneShotChannel<T>
}

impl<T> OneShotChannel<T> {
    /// Create a new one shot channel.
    fn new() -> Self {
        Self {
            is_ready: AtomicBool::new(false),
            in_use: AtomicBool::new(false),
            message: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    /// Split the channer into type safe instances that only allow for a single message
    /// to be sent and recieved.
    fn split(&mut self) -> (Sender<T>, Reciever<T>) {
        // reset the channel so that the message can be loaded.
        *self = Self::new();
        (
            Sender{
                channel: self
            },
            Reciever {
                channel: self
            }
        )

    } 

    /// Acquire load is_ready.
    fn is_ready(&self) -> bool {
        self.is_ready.load(Relaxed)
    }
}

impl<'a, T> Reciever<'a, T> {
    fn recieve(self) -> T {
        if !self.channel.is_ready.swap(false, Acquire) {
            panic!("Message is not ready yet.")
        }
        // We have checked and reset the ready flag so is safe.
        // Is consumed so will be called twice.
        unsafe {
                (*self.channel.message.get()).assume_init_read()
            }
    }
}

impl<'a, T> Sender<'a, T> {
    fn send(self, message: T) {
        // Is consumed on call so cannot be called twice.
        unsafe {
            (*self.channel.message.get()).write(message);
            self.channel.is_ready.store(true, Release);
        }
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