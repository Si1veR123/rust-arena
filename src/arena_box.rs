use std::{
    ops::{Deref, DerefMut}, ptr::NonNull, marker::PhantomData
};
use super::ArenaChunk;

/// A wrapper around box that points to memory allocated in an arena.
pub struct ArenaBox<'a, T, A: ArenaChunk> {
    inner: NonNull<T>,
    // Zero Sized Types don't belong to an arena chunk
    arena: Option<&'a A>,
    // arena box owns T
    phantom: PhantomData<T>
}

impl<'a, T, A: ArenaChunk> ArenaBox<'a, T, A> {
    /// Non-null pointer must be aligned, and point to a valid T
    pub unsafe fn new(arena: &'a A, object: NonNull<T>) -> Self {
        Self { inner: object, arena: Some(arena), phantom: PhantomData }
    }

    pub fn new_zero_sized() -> Self {
        Self { inner: NonNull::dangling(), arena: None, phantom: PhantomData }
    }

    /// Moves an object of type T out from the arena, and returns it
    pub fn into_inner(arena_box: ArenaBox<'a, T, A>) -> T {
        let ptr = arena_box.inner.as_ptr();

        // self isn't going to be dropped, so notify the arena that the allocation will be unused
        unsafe { arena_box.drop_notify_arena() };

        // don't run drop on self as it will call drop on T
        std::mem::forget(arena_box);

        unsafe { std::ptr::read(ptr) }
    }

    /// Returns a mut pointer to the T allocated in the arena.
    /// 
    /// Safety: pointer must not be used after the arena box is dropped
    pub unsafe fn mut_ptr(arena_box: &mut ArenaBox<'_, T, A>) -> *mut T {
        arena_box.inner.as_mut()
    }

    /// Returns a const pointer to the T allocated in the arena.
    /// 
    /// Safety: pointer must not be used after the arena box is dropped
    pub unsafe fn const_ptr(arena_box: &ArenaBox<'_, T, A>) -> *const T {
        arena_box.inner.as_ptr()
    }

    unsafe fn drop_notify_arena(&self) {
        // only adjust allocation count and drop T if T isn't a ZST
        if let Some(arena_ref) = self.arena { 
            arena_ref.adjust_allocation_count(-1);
        }
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
        unsafe {
            // safe to do when dropping self
            self.drop_notify_arena();

            // call T's destructor without deallocating the memory
            // this has the only pointer to T, and since this struct is being dropped, T can be dropped
            // safety: NonNull<T> is valid and properly aligned
            drop(std::ptr::read(self.inner.as_ptr()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Arena, ArenaAllocator};

    #[test]
    fn into_inner_test() {
        // test that into_inner only causes foo to be dropped once

        struct Foo(bool);
        impl Drop for Foo {
            fn drop(&mut self) {
                // the bool will be false if it hasnt been dropped yet
                assert!(!self.0);
                self.0 = true;
            }
        }

        let arena = Arena::new();
        let allocation = arena.allocate(Foo(false));
        let foo = ArenaBox::into_inner(allocation);
        // allocation (ArenaBox) is dropped here, Foo should not be dropped
        drop(foo);  // Foo is dropped here
    }

    #[test]
    fn drop_notify_arena_test() {
        let arena = Arena::new();
        
        let allocation = arena.allocate(1);
        assert_eq!(arena.chunks.last().unwrap().allocations.get(), 1);

        let second_allocation = arena.allocate(2);
        assert_eq!(arena.chunks.last().unwrap().allocations.get(), 2);

        drop(allocation);
        assert_eq!(arena.chunks.last().unwrap().allocations.get(), 1);

        drop(second_allocation);
        assert_eq!(arena.chunks.last().unwrap().allocations.get(), 0);
    }
}
