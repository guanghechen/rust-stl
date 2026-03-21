use collection::Collection;
use std::cmp::Ordering;

pub trait QueueLike<T>: Collection<Item = T> {
    fn front(&self) -> Option<&T>;
    fn enqueue(&mut self, element: T);
    fn dequeue(&mut self) -> Option<T>;

    fn enqueues<I>(&mut self, elements: I)
    where
        I: IntoIterator<Item = T>,
    {
        for element in elements {
            self.enqueue(element);
        }
    }

    fn replace_front(&mut self, new_back: T) -> Option<T> {
        let removed = self.dequeue();
        self.enqueue(new_back);
        removed
    }
}

pub trait DequeLike<T>: QueueLike<T> {
    fn back(&self) -> Option<&T>;
    fn enqueue_front(&mut self, element: T);
    fn dequeue_back(&mut self) -> Option<T>;

    fn enqueues_front<I>(&mut self, elements: I)
    where
        I: IntoIterator<Item = T>,
    {
        for element in elements {
            self.enqueue_front(element);
        }
    }

    fn replace_back(&mut self, new_front: T) -> Option<T> {
        let removed = self.dequeue_back();
        self.enqueue_front(new_front);
        removed
    }
}

pub trait CircularQueueLike<T>: DequeLike<T> {
    type Error;

    fn capacity(&self) -> usize;
    fn at(&self, index: isize) -> Option<&T>;
    fn resize(&mut self, new_capacity: usize) -> Result<(), Self::Error>;
    fn rearrange(&mut self);
}

pub trait PriorityQueueLike<T>: QueueLike<T> {
    fn compare(&self, a: &T, b: &T) -> Ordering;
}
