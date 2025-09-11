use crate::pack_span::{PackAccess, PackSpan, PackSpanMut, Part};

use super::PackOrder;

impl<'a, O: PackOrder> Iterator for PackSpan<'a, O> {
    type Item = Part;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(value) = self.get(0) {
            self.inner.consume(1, self.order.values_per_part());
            return Some(value);
        }
        None
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = usize::try_from(self.len()).ok();
        (size.unwrap_or(usize::MAX), size)
    }
}
impl<'a, O: PackOrder> ExactSizeIterator for PackSpan<'a, O> {}

impl<'a, O: PackOrder> Iterator for PackSpanMut<'a, O> {
    type Item = Part;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(value) = self.get(0) {
            self.inner.consume(1, self.order.values_per_part());
            return Some(value);
        }
        None
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = usize::try_from(self.len()).ok();
        (size.unwrap_or(usize::MAX), size)
    }
}
impl<'a, O: PackOrder> ExactSizeIterator for PackSpanMut<'a, O> {}
