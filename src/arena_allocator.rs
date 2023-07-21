use crate::single_chunk::SingleArena;

use super::ArenaChunk;
use super::ArenaAllocator;
use super::ArenaBox;
use super::chunk_linked_list::UnshrinkableLinkedList;

use std::mem::size_of;

const CHUNK_SIZE: usize = 4096;

pub struct Arena {
    pub(crate) chunks: UnshrinkableLinkedList<SingleArena>
}

impl Arena {
    /// # Safety
    /// UB if the constant CHUNK_SIZE is 0 and min_size is 0 (not very likely)
    unsafe fn new_chunk(&self, min_size: usize) {
        let chunk = SingleArena::new_unchecked(std::cmp::max(min_size, CHUNK_SIZE));
        self.chunks.push(chunk);
    }
}

impl ArenaAllocator<SingleArena> for Arena {
    fn new() -> Self {
        Self { chunks: UnshrinkableLinkedList::new() }
    }

    /// Allocate an object in an arena.
    /// 
    /// This may allocate on the heap if there is not enough capacity for the given object.
    fn allocate<T>(&self, object: T) -> ArenaBox<T, SingleArena> {
        let allocation_size = size_of::<T>();

        if allocation_size == 0 {
            return ArenaBox::new_zero_sized()
        }

        let chunk_opt = self.chunks.last();
        if let Some(chunk) = chunk_opt {
            let remaining_capacity = chunk.remaining_capacity();
            if allocation_size <= remaining_capacity {
                return unsafe { chunk.allocate_unchecked(object) }
            }
        }

        // create new chunk
        unsafe {
            self.new_chunk(size_of::<T>());
            let chunk = self.chunks.last().unwrap();
            return chunk.allocate_unchecked(object)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_sized_test() {
        let arena = Arena::new();

        let zst = ();
        assert_eq!(size_of::<()>(), 0);

        for _i in 0..1_000 {
            let _ = arena.allocate(zst.clone());
        }

        // no memory is actually allocated, therefore no chunks
        assert_eq!(arena.chunks.len(), 0);
    }

    #[test]
    fn allocate_three_chunks() {
        let integers_per_chunk = CHUNK_SIZE;
        let arena = Arena::new();

        for _i in 0..(integers_per_chunk*3) {
            let _ = arena.allocate(255u8);
        }

        assert_eq!(arena.chunks.len(), 3);
        assert!(arena.chunks.last().unwrap().remaining_capacity() < 8);
    }
}
