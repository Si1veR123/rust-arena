
pub unsafe fn read_memory_segment<'a, T: Into<*const u8>>(start_ptr: T, byte_length: usize) -> &'a [u8] {
    std::slice::from_raw_parts(start_ptr.into(), byte_length)
}

pub fn stress_heap_memory(alloc_count: usize) {
    let mut v = vec![];
    for _i in 0..alloc_count {
        let b = Box::new(5);
        v.push(b);
    }
}
