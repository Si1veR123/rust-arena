use std::{collections::LinkedList, cell::UnsafeCell};

pub(crate) struct UnshrinkableLinkedList<T> {
    inner: UnsafeCell<LinkedList<T>>,
}

impl<T> UnshrinkableLinkedList<T> {
    pub fn new() -> Self {
        Self { inner: UnsafeCell::new(LinkedList::new()) }
    }

    /// Unsafe as using push or extend will change the last value in the list.
    /// 
    /// Using this method may result in different items if the list is changed.
    pub unsafe fn last(&self) -> Option<&T> {
        (&*self.inner.get()).back()
    }

    pub fn push(&self, object: T) {
        // safety: only immutable references to this list are references to items in the list.
        // extending the list won't affect the immutable references
        unsafe { &mut *self.inner.get() }.push_back(object)
    }
}


impl<T> From<LinkedList<T>> for UnshrinkableLinkedList<T> {
    fn from(value: LinkedList<T>) -> Self {
        Self { inner: UnsafeCell::new(value)}
    }
}

impl<T: Clone> From<&[T]> for UnshrinkableLinkedList<T> {
    fn from(value: &[T]) -> Self {
        let mut linked_list = LinkedList::new();
        for item in value {
            linked_list.push_back(item.clone())
        }
        Self { inner: UnsafeCell::new(linked_list)}
    }
}

impl<A: Clone> FromIterator<A> for UnshrinkableLinkedList<A> {
    fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self {
        Self { inner: UnsafeCell::new(LinkedList::from_iter(iter))}
    }
}
