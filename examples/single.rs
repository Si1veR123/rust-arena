use arena_alloc::{single::SingleArena, ArenaChunk};
use std::time::Instant;

#[allow(dead_code)]
struct ComplicatedStruct {
    integer: i64,
    string: String,
    array: [u8; 4]
}

impl Default for ComplicatedStruct {
    fn default() -> Self {
        Self { integer: 4, string: String::from("test string"), array: [1, 2, 3, 4] }
    }
}

fn single_arena_test() {
    let start = Instant::now();
    let arena = SingleArena::new(10000000).unwrap();
    let mut stored = Vec::with_capacity(250000);
    for _i in 0..250000 {
        let allocation = arena.allocate(ComplicatedStruct::default()).unwrap();
        stored.push(allocation);
    }
    let end = Instant::now();

    println!("Arena took {:?}", end-start);
}

fn heap_test() {
    let start = Instant::now();
    let mut stored = Vec::with_capacity(250000);
    for _i in 0..250000 {
        let allocation = Box::new(ComplicatedStruct::default());
        stored.push(allocation);
    }
    let end = Instant::now();

    println!("Heap took {:?}", end-start);
}

fn main() {
    single_arena_test();
    heap_test();
}
