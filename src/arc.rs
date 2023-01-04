use std::{
    sync::atomic::{*, Ordering::*},
    ptr::NonNull,
    cell::UnsafeCell, 
    ops::{DerefMut, Deref}, 
    thread, 
    vec};


struct ArcData<T> {
    counter: AtomicUsize,
    data: T
}

//Arc should be send if and only if T is send and sync, the same hold for sync.
unsafe impl<T: Send + Sync> Send for Arc<T> {}
unsafe impl<T: Send + Sync> Sync for Arc<T> {}

pub struct Arc<T> {
    // Not using box as it ensures exclusive ownership.
    // This requires shared ownership.
    ptr: NonNull<ArcData<T>>
}

impl<T> Arc<T> {
    fn new(data: T) -> Self {
        Self {
            // Something new here, we allocate on the heap, leak the box to give up exclusive ownership.
            // Then create a nonNull from the leak, cool.
            ptr: NonNull::from(Box::leak(Box::new(
                ArcData {
                    counter: AtomicUsize::default(),
                    data
                }
            ))) 
        }
    }

    // Helper function as the compiler cannot ensure that the pointer is not null.
    // Only we can.
    fn data(&self) -> &ArcData<T> {
        unsafe { &self.ptr.as_ref()}
    }

    /// Get a mutable reference to the arc provided.
    /// This will return a reference when there is only one reference counted.
    /// exclusive ownership is ensured implicitly as &mut is passed in.
    fn get_mut(arc: &mut Self) -> Option<&mut T> {
        if arc.data().counter.load(Relaxed) == 1 {
            // There is only one reference so establish ownership and happens before relationship.
            fence(Acquire);

            Some( unsafe {
                &mut arc.ptr.as_mut().data    
            })
            
        } else {
            None
        }
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*&self.ptr.as_ref().data }
    }
}


impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        // Use threshholds as other threads can add concurrencly.
        if self.data().counter.fetch_add(1, Relaxed) > (usize::MAX / 2usize) {
            std::process::abort();
        };
        Self { ptr: self.ptr }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        if self.data().counter.fetch_sub(1, Release) == 1 {
            // Acquire ownership
            fence(Acquire);
            // Since we know that this is the only thread with ownership,
            // We can take exclusive ownership using Box::from_raw
            // And safely drop this
            unsafe { drop(Box::from_raw(self.ptr.as_ptr())) }
        }
    }
}