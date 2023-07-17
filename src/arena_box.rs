use std::{
    mem::ManuallyDrop,
    ops::{Deref, DerefMut}
};
use super::ArenaChunk;

/// A wrapper around box that points to memory allocated in an arena.
pub struct ArenaBox<'a, T, A: ArenaChunk> {
    inner: ManuallyDrop<Box<T>>,
    arena: &'a A
}

impl<'a, T, A: ArenaChunk> ArenaBox<'a, T, A> {
    pub fn new(arena: &'a A, boxed_object: Box<T>) -> Self {
        Self { inner: ManuallyDrop::new(boxed_object), arena: arena }
    }
}

impl<'a, T, A: ArenaChunk> Deref for ArenaBox<'a, T, A> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, T, A: ArenaChunk> DerefMut for ArenaBox<'a, T, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'a, T, A: ArenaChunk> Drop for ArenaBox<'a, T, A> {
    fn drop(&mut self) {
        self.arena.adjust_allocation_count(-1);
    }
}