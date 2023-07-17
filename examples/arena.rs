
use std::time::Instant;

use arena::{Arena, ArenaAllocator};

fn arena_test() {
    let start = Instant::now();

    let arena: Arena = Arena::new();
    let mut allocations = Vec::with_capacity(2500);
    
    for i in 0..2500 {
        allocations.push(arena.allocate(i));
    }

    assert!(**allocations.last().unwrap() == 2499);

    let end = Instant::now();
    println!("Arena took {:?}", end-start);
}

fn heap_test() {
    let start = Instant::now();

    let mut allocations = Vec::with_capacity(2500);

    for i in 0..2500 {
        allocations.push(Box::new(i));
    }

    assert!(**allocations.last().unwrap() == 2499);

    let end = Instant::now();
    println!("Heap took {:?}", end-start);
}

fn main() {
    arena_test();
    heap_test();
}
