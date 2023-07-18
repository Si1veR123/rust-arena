use std::alloc::Layout;
use std::mem::{size_of_val, align_of_val};
use std::alloc;
use std::ptr::NonNull;

use super::ArenaBox;

pub trait ArenaAllocator<C: ArenaChunk> {
    fn new() -> Self;
    fn allocate<T>(&self, object: T) -> ArenaBox<T, C>;
}

/// Objects implementing this trait can be used as a 'chunk' or 'block' in arena allocators
pub trait ArenaChunk: Sized {
    /// Create a new chunk without checking whether the size is valid
    /// 
    /// Can cause UB if size is 0
    unsafe fn new_unchecked(size: usize) -> Self;

    /// Allocate an object in the chunk.
    /// 
    /// Return None if the chunk doesn't have the capacity for the object.
    fn allocate<T>(&self, object: T) -> Option<ArenaBox<T, Self>>;

    /// Return a pointer to the start of the arena's memory.
    fn get_start_pointer_mut(&self) -> *mut u8;

    /// Return a pointer to the next place to write an object in the chunk.
    fn get_free_pointer_mut(&self) -> *mut u8;

    /// Set the free pointer to a new pointer.
    /// 
    /// UB if the pointer is set outside of the arena, or overwrites allocated objects.
    unsafe fn set_free_pointer(&self, ptr: *mut u8);

    /// The remaining capacity of the chunk in bytes.
    fn remaining_capacity(&self) -> usize;

    /// Adjust a counter of the number of allocations in the arena chunk.
    /// 
    /// This is handled in the allocation methods and when allocations are dropped.
    fn adjust_allocation_count(&self, count: isize);

    fn size(&self) -> usize;

    /// Create a new chunk, checking that size is greater than 0
    fn new(size: usize) -> Option<Self> {
        if size == 0 {
            None
        } else {
            Some(unsafe { Self::new_unchecked(size) })
        }
    }

    /// Allocate the memory needed for this chunk.
    /// 
    /// Returns a pointer to the start of the allocation.
    /// 
    /// UB if size is 0.
    /// Aborts process in an allocation error.
    unsafe fn intialise_chunk(size: usize) -> *mut u8 {
        // safety: align of one byte means that none of the checks are necessary
        // CAN BE UNSAFE IF SIZE IS 0
        let layout = Layout::from_size_align_unchecked(size, 1);
        let ptr = alloc::alloc(layout);
        if ptr.is_null() {
            alloc::handle_alloc_error(layout)
        }
        ptr
    }

    /// Allocate an object without checking:
    /// 
    /// * If it is a ZST
    /// 
    /// * If there is enough remaining capacity for the object
    unsafe fn allocate_unchecked<T>(&self, object: T) -> ArenaBox<T, Self> {
        let allocation_size = size_of_val(&object);
        let offset = self.get_free_pointer_mut().align_offset(align_of_val(&object));
        self.write_to_memory(object, allocation_size, offset)
    }

    /// Write a given object of size `byte_size` to memory at the free pointer.
    /// 
    /// Adjusts the free pointer and allocation count accordingly.
    /// 
    /// Free pointer + offset should be an aligned address for the object
    unsafe fn write_to_memory<'a, T>(&'a self, object: T, byte_size: usize, offset: usize) -> ArenaBox<'a, T, Self> {
        // write the object to memory at the free pointer
        // offset should make the allocation be aligned
        let object_pointer = self.get_free_pointer_mut().add(offset).cast::<T>();
        std::ptr::write(object_pointer, object);

        self.set_free_pointer(self.get_free_pointer_mut().add(byte_size + offset));

        self.adjust_allocation_count(1);
        
        // safety:: object pointer is non-null
        ArenaBox::new(&self, NonNull::new_unchecked(object_pointer))
    }

    /// Deallocate the memory used by the arena.
    /// 
    /// UB if used after deallocated.
    /// Memory is deallocated when the chunk is dropped.
    unsafe fn deallocate_arena(&mut self) {
        // safety: align of one byte means that none of the checks are necessary
        let layout = Layout::from_size_align_unchecked(self.size(), 1);
        // safety: memory in the arena will not have been deallocated, and layout is the same as size will not change
        // unsafe if the arena is dropped and attempted to be used again
        alloc::dealloc(self.get_start_pointer_mut(), layout);
    }
}
