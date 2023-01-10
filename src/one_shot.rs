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

unsafe impl<T: Send> Sync for OneShotChannel<T> {}

struct OneShotChannel<T> {
    is_ready: AtomicBool,
    message: UnsafeCell<MaybeUninit<T>>,
}

struct Sender<'a, T> {
    channel: &'a OneShotChannel<T>,
    reciever_thread: Thread,
}

struct Reciever<'a, T> {
    channel: &'a OneShotChannel<T>,
    no_send: PhantomData<*const ()>
}

impl<T> OneShotChannel<T> {
    /// Create a new one shot channel.
    fn new() -> Self {
        Self {
            is_ready: AtomicBool::new(false),
            message: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    /// Split the channer into type safe instances that only allow for a single message
    /// to be sent and recieved.
    fn split(&mut self, reciever_thread: Thread) -> (Sender<T>, Reciever<T>) {
        // reset the channel so that the message can be loaded.
        *self = Self::new();
        (
            Sender{
                channel: self,
                reciever_thread
            },
            Reciever {
                channel: self,
                no_send: PhantomData,
            }
        )
    } 

}

impl<'a, T> Reciever<'a, T> {
    fn recieve(self) -> T {
        
        if !self.channel.is_ready.swap(false, Acquire) {
            thread::park();
        }

        // We have checked and reset the ready flag so is safe.
        // Is consumed so cannot be called twice.
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
            
            // THis is ok because we have a phantom data on recieve which isnt Send.
            // Alas the Recieve type cannot be sent between threads and the reciveing_thread field will not change.
            self.reciever_thread.unpark();
        }
    }
}

impl<T> Drop for OneShotChannel<T> {
    fn drop(&mut self) {
        // if we can get a mutable reference here then we have exclusive ownership.
        if *self.is_ready.get_mut() {
            unsafe {
                ()
                // Drop the message to avoid memory leaks.
                // Doesnt assune init read drop the contents?
                //self.message.get_mut().assume_init_drop()
            }
        }
    }
}

#[test]
fn test_one_shot() {
    let mut channel: OneShotChannel<String> = OneShotChannel::new();

    thread::scope(|s| {
        let (send, recieve) = channel.split(thread::current());
        s.spawn(move || {
            // Send a message
            send.send(String::from("hello world!"));    
        });

        assert_eq!(recieve.recieve(), "hello world!");
    });
}