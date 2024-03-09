use std::collections::VecDeque;

/// A buffer with fixed size. When pushing exceeds its size, the oldest / first item is removed.
#[derive(Debug, Clone)]
pub struct FixedSizeBuffer<T> {
    inner: VecDeque<T>,
    size: usize,
}

impl<T> FixedSizeBuffer<T> {
    pub fn new(size: usize) -> Self {
        Self {
            inner: VecDeque::new(),
            size,
        }
    }

    pub fn add(&mut self, item: T) -> Option<T> {
        let removed = if self.size <= self.inner.len() {
            self.inner.pop_front()
        } else {
            None
        };

        self.inner.push_back(item);

        removed
    }

    pub fn remove(&mut self) -> Option<T> {
        self.inner.pop_front()
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn first(&self) -> Option<&T> {
        self.inner.front()
    }

    pub fn last(&self) -> Option<&T> {
        self.inner.get(self.inner.len().saturating_sub(1))
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn iter(&self) -> FixedSizeBufferIter<'_, T> {
        self.into_iter()
    }
}

#[derive(Debug, Clone)]
pub struct FixedSizeBufferIter<'a, T> {
    buf: &'a FixedSizeBuffer<T>,
    i: usize,
    m: usize,
}

impl<'a, T> Iterator for FixedSizeBufferIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i < self.m {
            self.i += 1;
            Some(&self.buf.inner[self.i - 1])
        } else {
            None
        }
    }
}

impl<'a, T> IntoIterator for &'a FixedSizeBuffer<T> {
    type Item = &'a T;
    type IntoIter = FixedSizeBufferIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        FixedSizeBufferIter {
            buf: self,
            i: 0,
            m: self.len(),
        }
    }
}

impl<T> Extend<T> for FixedSizeBuffer<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for new in iter.into_iter() {
            let _ = self.add(new);
        }
    }
}
