use std::cell::Cell;
use std::mem::{size_of_val, align_of_val};
use std::ops::Add;
use std::ptr::NonNull;
use std::sync::{Mutex, MutexGuard};
use std::sync::Arc;

use super::arena_trait::ArenaChunk;
use super::ArenaBox;

/// A single 'chunk' or 'block' of allocated memory.
/// 
/// The chunk has a constant size, and only allocates memory once, when creating the chunk.
/// This means that allocations can fail if there is no capacity remaining.
pub struct SingleArena {
    size: usize,
    start_pointer: *mut u8,
    free_pointer: Cell<*mut u8>,
    pub allocations: Cell<usize>
}

impl ArenaChunk for SingleArena {
    unsafe fn new_unchecked(size: usize) -> Self {
        let allocation = Self::intialise_chunk(size);
        Self { size, start_pointer: allocation, free_pointer: Cell::new(allocation), allocations: Cell::new(0) }
    }

    fn allocate<T>(&self, object: T) -> Option<ArenaBox<T, Self>> {
        let allocation_size = size_of_val(&object);

        // handle zst
        if allocation_size == 0 {
            return Some(ArenaBox::new_zero_sized())
        }

        let offset = self.get_free_pointer_mut().align_offset(align_of_val(&object));

        // checks that there is enough free space to allocate this object
        if allocation_size.checked_add(offset)? <= self.remaining_capacity() {
            // safety: byte size is greater or equal to allocation size,
            // and there is enough remaining capacity to store the object.
            unsafe { Some(self.write_to_memory(object, allocation_size, offset)) }
        } else {
            None
        }
    }

    #[inline]
    fn get_start_pointer_mut(&self) -> *mut u8 {
        self.start_pointer
    }

    #[inline]
    fn get_free_pointer_mut(&self) -> *mut u8 {
        self.free_pointer.get()
    }

    unsafe fn set_free_pointer(&self, ptr: *mut u8) {
        self.free_pointer.set(ptr)
    }

    fn remaining_capacity(&self) -> usize {
        (self.start_pointer as usize + self.size) - self.free_pointer.get() as usize
    }

    fn adjust_allocation_count(&self, count: isize) {
        self.allocations.set(self.allocations.get().checked_add_signed(count).expect("Allocation count overflow (too many allocations)"))
    }

    #[inline]
    fn size(&self) -> usize {
        self.size
    }
}

impl Drop for SingleArena {
    fn drop(&mut self) {
        // drop means that there are no other references to the chunk, it can be safely deallocated.
        unsafe {
            self.deallocate_arena()
        }
    }
}


/// Same as `SingleArena`, however is thread safe.
/// 
/// The arena can be cloned without allocating new memory, see atomic_single example.
#[derive(Clone)]
pub struct AtomicSingleArena {
    size: usize,
    // raw pointers aren't send + sync, so easiest way to make the struct send + sync is represent the pointer as a usize
    start_pointer: usize,
    free_pointer: Arc<Mutex<usize>>,
    allocations: Arc<Mutex<usize>>
}

impl AtomicSingleArena {
    /// Similar to write_to_memory, however uses a mutex lock on the free pointer
    unsafe fn write_to_memory_with_lock<T>(&self, mut ptr_lock: MutexGuard<'_, usize>, object: T, byte_size: usize, offset: usize) -> ArenaBox<T, Self> {
        let ptr = (*ptr_lock as *mut u8).add(offset);

        // write the object to memory at the free pointer
        let object_pointer = ptr.cast::<T>();
        std::ptr::write(object_pointer, object);

        *ptr_lock = ptr_lock.add(byte_size + offset);
        self.adjust_allocation_count(1);

        // safety: free pointer and therefore object pointer is non-null
        ArenaBox::new(&self, NonNull::new_unchecked(object_pointer))
    }
}

impl ArenaChunk for AtomicSingleArena {
    unsafe fn new_unchecked(size: usize) -> Self {
        let allocation = Self::intialise_chunk(size);
        Self { size, start_pointer: allocation as usize, free_pointer: Arc::new(Mutex::new(allocation as usize)), allocations: Arc::new(Mutex::new(0))}
    }

    fn allocate<T>(&self, object: T) -> Option<ArenaBox<T, Self>> {
        let allocation_size = size_of_val(&object);

        // handle zst
        if allocation_size == 0 {
            return Some(ArenaBox::new_zero_sized())
        }

        let ptr_lock = self.free_pointer.lock().ok()?;
        let offset = (*ptr_lock as *mut u8).align_offset(align_of_val(&object));

        // checks that there is enough free space to allocate this object
        if ptr_lock.checked_add(allocation_size)? <= self.start_pointer + self.size {
            // safety: byte size is greater or equal to allocation size,
            // and there is enough remaining capacity to store the object.
            // the free pointer is a valid pointer.
            unsafe { Some(self.write_to_memory_with_lock(ptr_lock, object, allocation_size, offset)) }
        } else {
            None
        }
    }

    unsafe fn write_to_memory<T>(&self, object: T, byte_size: usize, offset: usize) -> ArenaBox<T, Self> {
        let ptr_lock = self.free_pointer.lock().expect("Error locking mutex in Atomic Single Arena");
        self.write_to_memory_with_lock(ptr_lock, object, byte_size, offset)
    }

    fn get_start_pointer_mut(&self) -> *mut u8 {
        self.start_pointer as *mut u8
    }

    /// Returns a pointer to the next place to write an object to the chunk.
    /// 
    /// Requires acquiring a mutex lock.
    fn get_free_pointer_mut(&self) -> *mut u8 {
        *self.free_pointer.lock().expect("Error locking mutex in Atomic Single Arena") as *mut u8
    }

    unsafe fn set_free_pointer(&self, ptr: *mut u8) {
        let mut lock = self.free_pointer.lock().expect("Error locking mutex in Atomic Single Arena");
        *lock = ptr as usize;
    }

    fn remaining_capacity(&self) -> usize {
        (self.start_pointer + self.size) - self.get_free_pointer_mut() as usize
    }

    fn adjust_allocation_count(&self, count: isize) {
        let mut lock = self.allocations.lock().expect("Error locking mutex in Atomic Single Arena");
        *lock = lock.checked_add_signed(count).expect("Allocation count overflow (too many allocations)");
    }

    fn size(&self) -> usize {
        self.size
    }
}


impl Drop for AtomicSingleArena {
    fn drop(&mut self) {
        let remaining_arena_copies = Arc::strong_count(&self.free_pointer);
        if remaining_arena_copies == 1 {
            // safety: there are no more references to the arena except the one being dropped. the arena can be deallocated.
            unsafe { self.deallocate_arena() }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn single_allocation() {
        let expected_slice: Vec<u8> = [0u8; 100].iter().enumerate().map(|(i, _e)| i as u8).collect();

        let arena = SingleArena::new(100).unwrap();
        let start_ptr = arena.get_free_pointer_mut();
        for i in 0..100_u8 {
            let _ = arena.allocate(i).unwrap();
        }

        let arena_values = unsafe { std::slice::from_raw_parts(start_ptr.cast_const(), 100) };
        assert_eq!(expected_slice.as_slice(), arena_values);
    }

    #[test]
    fn atomic_single_allocation() {
        let arena = AtomicSingleArena::new(64).unwrap();
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
        let arena_values = unsafe { std::slice::from_raw_parts(start_ptr.cast_const(), 64) };
        for val in arena_values.iter().cloned() {
            assert!(val == 10 || val == 20)
        }
        
        // keep alive until the end
        drop(arena);
    }
}
