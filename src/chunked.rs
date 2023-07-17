use super::ArenaChunk;
use super::ArenaAllocator;
use super::ArenaBox;

use std::cell::UnsafeCell;
use std::collections::LinkedList;
use std::mem::size_of_val;

const CHUNK_SIZE: usize = 4096;

pub struct Arena<C: ArenaChunk> {
    // UnsafeCell is required as allocations given out will have a read-only reference to an arena chunk,
    // however allocating requires a mutable reference to the list of chunks. The arena guarantees that the
    // reference to the arena chunk will live as long as the arena, as no arena chunks are dropped. This means
    // that the list of chunks can be mutated through unsafe cell, as long as the read-only references are still valid.
    // Therefore mutating the list shouldn't delete any entries.
    chunks: UnsafeCell<LinkedList<C>>
}

impl<C: ArenaChunk> Arena<C> {
    unsafe fn new_chunk(&self, min_size: usize) {
        let chunk = C::new_unchecked(std::cmp::max(min_size, CHUNK_SIZE));
        (&mut *self.chunks.get()).push_back(chunk);
    }
}

impl<C: ArenaChunk> ArenaAllocator<C> for Arena<C> {
    fn new() -> Self {
        Self { chunks: UnsafeCell::new(LinkedList::new()) }
    }

    fn allocate<'a, T>(&'a self, object: T) -> ArenaBox<'a, T, C> {
        let allocation_size = size_of_val(&object);

        let chunks = unsafe { &*self.chunks.get() };
        let chunk_opt = chunks.back();
        if let Some(chunk) = chunk_opt {
            let remaining_capacity = chunk.remaining_capacity();
            if allocation_size <= remaining_capacity {
                return unsafe { chunk.allocate_unchecked(object) }
            }
        }

        // create new chunk
        unsafe {
            println!("ALLOCATING");
            self.new_chunk(size_of_val(&object));
            let chunk = chunks.back().unwrap();
            return chunk.allocate_unchecked(object)
        }
    }
}
