use crate::single::SingleArena;

use super::ArenaChunk;
use super::ArenaAllocator;
use super::ArenaBox;
use super::chunk_linked_list::UnshrinkableLinkedList;

use std::mem::size_of_val;

const CHUNK_SIZE: usize = 4096;

pub struct Arena {
    // UnsafeCell is required as allocations given out will have a read-only reference to an arena chunk,
    // however allocating requires a mutable reference to the list of chunks. The arena guarantees that the
    // reference to the arena chunk will live as long as the arena, as no arena chunks are dropped. This means
    // that the list of chunks can be mutated through unsafe cell, as long as the read-only references are still valid.
    // Therefore mutating the list shouldn't delete any entries.
    chunks: UnshrinkableLinkedList<SingleArena>
}

impl Arena {
    unsafe fn new_chunk(&self, min_size: usize) {
        let chunk = SingleArena::new_unchecked(std::cmp::max(min_size, CHUNK_SIZE));
        self.chunks.push(chunk);
    }
}

impl ArenaAllocator<SingleArena> for Arena {
    fn new() -> Self {
        Self { chunks: UnshrinkableLinkedList::new() }
    }

    fn allocate<'a, T>(&'a self, object: T) -> ArenaBox<'a, T, SingleArena> {
        let allocation_size = size_of_val(&object);

        let chunk_opt = unsafe { self.chunks.last() };
        if let Some(chunk) = chunk_opt {
            let remaining_capacity = chunk.remaining_capacity();
            if allocation_size <= remaining_capacity {
                return unsafe { chunk.allocate_unchecked(object) }
            }
        }

        // create new chunk
        unsafe {
            self.new_chunk(size_of_val(&object));
            let chunk = self.chunks.last().unwrap();
            return chunk.allocate_unchecked(object)
        }
    }
}
