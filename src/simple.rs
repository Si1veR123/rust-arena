use std::alloc::{self, Layout};
use std::cell::Cell;
use std::mem::size_of_val;
use std::fmt::Debug;
use std::ops::Add;
use std::sync::{Mutex, MutexGuard};
use std::sync::Arc;
use super::arena::{ArenaAllocator, ArenaBox};
use super::misc::read_memory_segment;

pub struct SimpleArena {
    size: usize,
    start_pointer: *mut u8,
    free_pointer: Cell<*mut u8>
}

impl ArenaAllocator for SimpleArena {
    unsafe fn new_unchecked(size: usize) -> Self {
        let allocation = Self::intialise_arena(size);
        Self { size, start_pointer: allocation, free_pointer: Cell::new(allocation) }
    }

    fn allocate<T>(&self, object: T) -> Option<ArenaBox<T, Self>> {
        let allocation_size = size_of_val(&object);
        unsafe {
            // safety: free pointer is guaranteed to be within the arena, provided that no unchecked allocations have been made
            //         start pointer is guaranteed to be within the arena
            // checks that there is enough free space to allocate this object
            if self.free_pointer.get().add(allocation_size) <= self.start_pointer.add(self.size) {
                Some(self.write_to_memory(object, allocation_size))
            } else {
                None
            }
        }
    }

    fn get_free_pointer_mut(&self) -> *mut u8 {
        self.free_pointer.get()
    }

    unsafe fn set_free_pointer(&self, ptr: *mut u8) {
        self.free_pointer.set(ptr)
    }

    unsafe fn deallocate_arena(&mut self) {
        // safety: align of one byte means that none of the checks are necessary
        let layout = Layout::from_size_align_unchecked(self.size, 1);
        // safety: memory in the arena will not have been deallocated, and layout is the same as size will not change
        // unsafe if the arena is dropped and attempted to be used again
        alloc::dealloc(self.start_pointer, layout);
    }
}

impl Drop for SimpleArena {
    fn drop(&mut self) {
        unsafe {
            self.deallocate_arena()
        }
    }
}

impl Debug for SimpleArena {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let segment = unsafe { read_memory_segment(self.start_pointer.cast_const(), self.size) };
        write!(f, "Arena values: {:?}", segment)
    }
}

#[derive(Clone)]
pub struct AtomicSimpleArena {
    size: usize,
    // raw pointers aren't send + sync, so easiest way to make the struct send + sync is represent the pointer as a usize
    start_pointer: usize,
    free_pointer: Arc<Mutex<usize>>
}

impl AtomicSimpleArena {
    // similar to write_to_memory, however uses a mutex lock on the free pointer
    unsafe fn write_to_memory_with_lock<T>(&self, mut ptr_lock: MutexGuard<'_, usize>, object: T, byte_size: usize) -> ArenaBox<T, Self> {
        let ptr = *ptr_lock as *mut u8;

        // write the object to memory at the free pointer
        let object_pointer = ptr.cast::<T>();
        std::ptr::write(object_pointer, object);
        let boxed_object = Box::from_raw(object_pointer);

        *ptr_lock = ptr_lock.add(byte_size);
        ArenaBox::new(boxed_object)
    }
}

impl ArenaAllocator for AtomicSimpleArena {
    unsafe fn new_unchecked(size: usize) -> Self {
        let allocation = Self::intialise_arena(size);
        Self { size, start_pointer: allocation as usize, free_pointer: Arc::new(Mutex::new(allocation as usize)) }
    }

    fn allocate<T>(&self, object: T) -> Option<ArenaBox<T, Self>> {
        let allocation_size = size_of_val(&object);
        let ptr_lock = self.free_pointer.lock().ok()?;
        unsafe {
            // safety: free pointer is guaranteed to be within the arena, provided that no unchecked allocations have been made
            //         start pointer is guaranteed to be within the arena
            // checks that there is enough free space to allocate this object
            if ptr_lock.add(allocation_size) <= self.start_pointer + self.size {
                Some(self.write_to_memory_with_lock(ptr_lock, object, allocation_size))
            } else {
                None
            }
        }
    }

    unsafe fn write_to_memory<T>(&self, object: T, byte_size: usize) -> ArenaBox<T, Self> {
        let ptr_lock = self.free_pointer.lock().expect("Error locking mutex in Atomic Simple Arena");
        self.write_to_memory_with_lock(ptr_lock, object, byte_size)
    }

    fn get_free_pointer_mut(&self) -> *mut u8 {
        *self.free_pointer.lock().expect("Error locking mutex in Atomic Simple Arena") as *mut u8
    }

    unsafe fn set_free_pointer(&self, ptr: *mut u8) {
        let mut lock = self.free_pointer.lock().expect("Error locking mutex in Atomic Simple Arena");
        *lock = ptr as usize;
    }

    unsafe fn deallocate_arena(&mut self) {
        // safety: align of one byte means that none of the checks are necessary
        let layout = Layout::from_size_align_unchecked(self.size, 1);
        // safety: memory in the arena will not have been deallocated, and layout is the same as size will not change
        // unsafe if the arena is dropped and attempted to be used again
        alloc::dealloc(self.start_pointer as *mut u8, layout);
    }
}


impl Drop for AtomicSimpleArena {
    fn drop(&mut self) {
        let remaining_arena_copies = Arc::strong_count(&self.free_pointer);
        if remaining_arena_copies == 1 {
            // safety: there are no more references to the arena except the one being dropped. the arena can be deallocated.
            unsafe { self.deallocate_arena() }
        }
    }
}

impl Debug for AtomicSimpleArena {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let segment = unsafe { read_memory_segment((self.start_pointer as *mut u8).cast_const(), self.size) };
        write!(f, "Arena values: {:?}", segment)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn simple_allocation() {
        let expected_slice: Vec<u8> = [0u8; 100].iter().enumerate().map(|(i, _e)| i as u8).collect();

        let arena = SimpleArena::new(100).unwrap();
        let start_ptr = arena.get_free_pointer_mut();
        for i in 0..100_u8 {
            let _ = arena.allocate(i).unwrap();
        }

        let arena_values = unsafe { read_memory_segment(start_ptr.cast_const(), 100) };
        assert_eq!(expected_slice.as_slice(), arena_values);
    }


    #[test]
    fn atomic_simple_allocation() {
        let arena = AtomicSimpleArena::new(64).unwrap();
        let start_ptr = arena.get_free_pointer_mut();
        let arena_2 = arena.clone();
        let arena_3 = arena.clone();

        let thread1 = thread::spawn(move || {
            for _i in 0..32 {
                arena_2.allocate(10_i8);
            }
        });

        let thread2 = thread::spawn(move || {
            for _i in 0..32 {
                arena_3.allocate(20_i8);
            }
        });

        let _ = thread1.join();
        let _ = thread2.join();

        // threads have allocated all 64 bytes
        // further allocations should fail
        assert!(arena.allocate(0).is_none());

        // all values should be 10 or 20
        let arena_values = unsafe { read_memory_segment(start_ptr.cast_const(), 64) };
        for val in arena_values.iter().cloned() {
            assert!(val == 10 || val == 20)
        }
    }
}
