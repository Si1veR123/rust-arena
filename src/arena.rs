use std::alloc::Layout;
use std::mem::{ManuallyDrop, size_of_val};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::alloc;

pub trait ArenaAllocator
    where Self: Sized {
    // create a new arena without checking whether the size is valid
    unsafe fn new_unchecked(size: usize) -> Self;

    // allocate an object in the arena. could return None if the arena doesn't have the capacity for the object
    fn allocate<T>(&self, object: T) -> Option<ArenaBox<T, Self>>;

    // deallocate the memory used by the arena
    // UB if used before dropped
    unsafe fn deallocate_arena(&mut self);

    // return a pointer to the next place to write an object
    fn get_free_pointer_mut(&self) -> *mut u8;

    // set the free pointer to a new address
    unsafe fn set_free_pointer(&self, ptr: *mut u8);

    fn new(size: usize) -> Option<Self> {
        if size == 0 {
            None
        } else {
            Some(unsafe { Self::new_unchecked(size) })
        }
    }

    unsafe fn intialise_arena(size: usize) -> *mut u8 {
        // safety: align of one byte means that none of the checks are necessary
        // CAN BE UNSAFE IF SIZE IS 0
        let layout = Layout::from_size_align_unchecked(size, 1);
        alloc::alloc(layout)
    }

    unsafe fn allocate_unchecked<T>(&self, object: T) -> ArenaBox<T, Self> {
        let allocation_size = size_of_val(&object);
        self.write_to_memory(object, allocation_size)
    }

    unsafe fn write_to_memory<T>(&self, object: T, byte_size: usize) -> ArenaBox<T, Self> {
        // write the object to memory at the free pointer
        let object_pointer = self.get_free_pointer_mut().cast::<T>();
        let _ = std::ptr::write(object_pointer, object);
        let boxed_object = Box::from_raw(object_pointer);
        
        self.set_free_pointer(self.get_free_pointer_mut().add(byte_size));
        ArenaBox::new(boxed_object)
    }
}

pub struct ArenaBox<'a, T, A: ArenaAllocator> {
    inner: ManuallyDrop<Box<T>>,
    arena: PhantomData<&'a A>
}

impl<'a, T, A: ArenaAllocator> ArenaBox<'a, T, A> {
    pub fn new(boxed_object: Box<T>) -> Self {
        Self { inner: ManuallyDrop::new(boxed_object), arena: PhantomData }
    }
}

impl<'a, T, A: ArenaAllocator> Deref for ArenaBox<'a, T, A> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, T, A: ArenaAllocator> DerefMut for ArenaBox<'a, T, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

