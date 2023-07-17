use arena_alloc::single::AtomicSingleArena;
use arena_alloc::ArenaChunk;
use std::thread;


fn main() {
    let arena = AtomicSingleArena::new(64).unwrap();
    let arena_2 = arena.clone();
    let arena_3 = arena.clone();

    println!("{:?}", arena);

    let thread1 = thread::spawn(move || {
        for _i in 0..32 {
            arena_2.allocate(10_i8);
        }
    });

    let thread2 = thread::spawn(move || {
        for _i in 0..32 {
            arena_3.allocate(20_i8);
        }
    });

    let _ = thread1.join();
    let _ = thread2.join();

    println!("{:?}", arena);

    // threads have allocated all 64 bytes
    // further allocations should fail
    assert!(arena.allocate(0).is_none());
}
