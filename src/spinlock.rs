
use std::{sync::atomic::{*, Ordering::*}, cell::UnsafeCell, ops::DerefMut, thread, vec};
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

#[test]
fn it_works() {
    let lock: SpinLock<Vec<u64>> = SpinLock::new(Vec::new());
    thread::scope(|s| {
        s.spawn(|| {
            // Lock and aquire the guard
            // We can simply push the guard as we have implemented DerefMut (COOL)
            lock.lock().push(1u64);
            // Drop the guard and release the lock (done implicitly)
        });
        s.spawn(|| {
            // This can be dome condurrenclty
            let mut g = lock.lock();
            g.push(2u64);
        });
    });
    let g = lock.lock();
    assert!(g.as_slice() == [1, 2] || g.as_slice() == [2, 1]);
    drop(g);
}