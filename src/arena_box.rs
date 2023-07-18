use std::{
    ops::{Deref, DerefMut}, ptr::NonNull
};
use super::ArenaChunk;

/// A wrapper around box that points to memory allocated in an arena.
pub struct ArenaBox<'a, T, A: ArenaChunk> {
    inner: NonNull<T>,
    arena: &'a A
}

impl<'a, T, A: ArenaChunk> ArenaBox<'a, T, A> {
    /// Non-null pointer must be aligned, and point to a valid T
    pub unsafe fn new(arena: &'a A, object: NonNull<T>) -> Self {
        Self { inner: object, arena }
    }
}

impl<'a, T, A: ArenaChunk> Deref for ArenaBox<'a, T, A> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        // safety: shared reference to self allows a shared reference to the inner T
        unsafe { self.inner.as_ref() }
    }
}

impl<'a, T, A: ArenaChunk> DerefMut for ArenaBox<'a, T, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // safety: unique reference to self allows a unique reference to the inner T
        unsafe { self.inner.as_mut() }
    }
}

impl<'a, T, A: ArenaChunk> Drop for ArenaBox<'a, T, A> {
    fn drop(&mut self) {
        self.arena.adjust_allocation_count(-1);
        // call T's destructor without deallocating the memory
        // this has the only pointer to T, and since this struct is being dropped, T can be dropped
        // safety: NonNull<T> is valid and properly aligned
        unsafe { drop(std::ptr::read(self.inner.as_ptr())) }
    }
}