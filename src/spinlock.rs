
use std::{sync::atomic::{*, Ordering::*}, cell::UnsafeCell, ops::DerefMut};
use std::ops::Deref;

pub struct Guard<'a ,T> {
    lock: &'a SpinLock<T>
}

impl<T> Deref for Guard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        // Since this guard is only available when a lock is aquired this code is safe.
        unsafe {
            &*self.lock.data.get()
        }
    }
}

impl<T> DerefMut for Guard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            &mut *self.lock.data.get()
        }
    }    
}

impl<T> Drop for Guard<'_, T> {
    fn drop(&mut self) {
        // When we drop the guard we give up the lock to other threads.
        // Todo: wake other threads if they are waiting.
        self.lock.locked.store(false, Release);
    }
}


/// Used to protect data when multiple threads are concurrently accessing.
pub struct SpinLock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>
}

// There is no need for T to be sync as the lock will ensure only a single access
unsafe impl<T> Sync for SpinLock<T> where T: Send{}

impl<T> SpinLock<T> {
    fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data)
        }
    }

    fn lock(&self) -> Guard<T> {
        // Continue to try and lock if locked is true and tell the compiler its in a spin loop.
        while self.locked.swap(true, Acquire) == true {
            std::hint::spin_loop()
        }
        
        Guard {
            lock: &self
        }
        
    }

    fn unlock(&self) {
        self.locked.store(false, Release)
    }
}