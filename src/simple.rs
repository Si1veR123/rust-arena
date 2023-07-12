use std::alloc::{self, Layout};
use std::cell::Cell;
use std::marker::PhantomData;
use std::mem::{size_of_val, ManuallyDrop};
use std::ops::{Deref, DerefMut};
use std::fmt::Debug;
use super::misc::read_memory_segment;

pub struct SimpleArena {
    size: usize,
    start_pointer: *mut u8,
    free_pointer: Cell<*mut u8>
}

impl SimpleArena {
    pub fn new(size: usize) -> Self {
        // size in bytes
        let allocation;
        unsafe {
            // safety: align of one byte means that none of the checks are necessary
            let layout = Layout::from_size_align_unchecked(size, 1);
            allocation = alloc::alloc(layout);
        }
        Self { size, start_pointer: allocation, free_pointer: Cell::new(allocation) }
    }

    pub fn allocate<T>(&self, object: T) -> Option<ArenaBox<T>> {
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

    pub unsafe fn allocate_unchecked<T>(&self, object: T) -> ArenaBox<T> {
        let allocation_size = size_of_val(&object);
        self.write_to_memory(object, allocation_size)
    }

    unsafe fn write_to_memory<T>(&self, object: T, byte_size: usize) -> ArenaBox<T> {
        // write the object to memory at the free pointer
        let boxed_object;
        let object_pointer = self.free_pointer.get().cast::<T>();
        let _ = std::mem::replace(&mut *object_pointer, object);
        boxed_object = Box::from_raw(object_pointer);
        self.free_pointer.set(self.free_pointer.get().add(byte_size));
        ArenaBox::new(boxed_object)
    }

    pub fn get_start_pointer(&self) -> *const u8 {
        self.start_pointer.cast_const()
    }

    pub fn get_free_pointer(&self) -> *const u8 {
        self.free_pointer.get().cast_const()
    }

}

impl Drop for SimpleArena {
    fn drop(&mut self) {
        unsafe {
            // safety: align of one byte means that none of the checks are necessary
            let layout = Layout::from_size_align_unchecked(self.size, 1);
            // safety: memory in the arena will not have been deallocated, and layout is the same as size will not change
            alloc::dealloc(self.start_pointer, layout);
        }
    }
}

impl Debug for SimpleArena {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let segment = unsafe { read_memory_segment(self.start_pointer.cast_const(), self.size) };
        write!(f, "Arena values: {:?}", segment)
    }
}

pub struct ArenaBox<'a, T> {
    inner: ManuallyDrop<Box<T>>,
    arena: PhantomData<&'a SimpleArena>
}

impl<'a, T> ArenaBox<'a, T> {
    pub fn new(boxed_object: Box<T>) -> Self {
        Self { inner: ManuallyDrop::new(boxed_object), arena: PhantomData }
    }
}

impl<'a, T> Deref for ArenaBox<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, T> DerefMut for ArenaBox<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
