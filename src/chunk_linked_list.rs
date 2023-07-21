use std::{collections::LinkedList, cell::UnsafeCell};

/// This list allows references to elements in the list and pushing elements to the end of the list, with a shared reference.
/// 
/// This is allowed as a linked list ensures that the item references remain valid, and this list doesn't allow the removal or moving of items.
/// 
/// Used to store a list of memory blocks in an arena.
pub(crate) struct UnshrinkableLinkedList<T> {
    inner: UnsafeCell<LinkedList<T>>,
}

impl<T> UnshrinkableLinkedList<T> {
    pub fn new() -> Self {
        Self { inner: UnsafeCell::new(LinkedList::new()) }
    }

    /// Using this method may result in different items if the list is changed, using interior mutability.
    pub fn last(&self) -> Option<&T> {
        // safety: unsafe cell has a valid and dereferenceable pointer,
        // and no mutable references are released to the linked list
        unsafe { (*self.inner.get()).back() }
    }

    /// Using this method may result in different items if the list is changed, using interior mutability.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        // safety: unsafe cell has a valid and dereferenceable pointer,
        // and no mutable references are released to the linked list
        unsafe { (*self.inner.get()).len() }
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
