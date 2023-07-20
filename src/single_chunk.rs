use std::cell::Cell;
use std::mem::{size_of, align_of};

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
        let allocation_size = size_of::<T>();

        // handle zst
        if allocation_size == 0 {
            return Some(ArenaBox::new_zero_sized())
        }

        let offset = self.get_free_pointer_mut().align_offset(align_of::<T>());

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


#[cfg(test)]
mod tests {
    use super::*;

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
}
