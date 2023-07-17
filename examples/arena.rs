
use arena_alloc::{chunked::Arena, ArenaAllocator, single::SingleArena};

trait Sound {
    fn quack(&self);
}

struct Duck {
    sound: String
}

impl Sound for Duck {
    fn quack(&self) {
        println!("{}", &self.sound);
    }
}

fn main() {
    let arena: Arena<SingleArena> = Arena::new();
    
    for i in 0..5000 {
        arena.allocate(i);
    }
}
