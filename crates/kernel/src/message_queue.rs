use std::collections::VecDeque;

pub struct MessageQueue<M> {
    queue: VecDeque<M>,
    capacity: usize,
}

impl<M> MessageQueue<M> {
    pub fn with_capacity(n: usize) -> Self {
        Self {
            queue: VecDeque::with_capacity(n),
            // This is still needed to determine the maxiumum messages of queue behavior
            capacity: n,
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn size(&self) -> usize {
        self.queue.len()
    }

    pub fn push(&mut self, message: M) {
        self.queue.push_back(message);
    }

    pub fn pop(&mut self) -> Option<M> {
        self.queue.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use googletest::prelude::*;

    #[googletest::test]
    fn queue_is_created_with_maximum_capacity() {
        let queue: MessageQueue<()> = MessageQueue::with_capacity(10);
        expect_that!(queue.capacity(), eq(10));
        expect_that!(queue.size(), eq(0));
    }

    #[googletest::test]
    fn message_can_be_pushed_to_queue() {
        let mut queue = MessageQueue::with_capacity(10);
        let message = "message 1".to_string();

        queue.push(message);

        expect_that!(queue.size(), eq(1));
    }

    #[googletest::test]
    fn message_can_be_popped_to_queue() {
        let mut queue = MessageQueue::with_capacity(10);
        let message = "message 1".to_string();
        queue.push(message);

        expect_that!(queue.pop(), pat!(Some(eq("message 1"))));
        expect_that!(queue.pop(), pat!(None));
        expect_that!(queue.size(), eq(0));
    }
}
