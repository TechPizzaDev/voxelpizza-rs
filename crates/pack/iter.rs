use super::{
    order::PackOrder,
    part::Part,
    span::{PackAccess, PackSpan, PackSpanMut},
};

// TODO: "align_to" methods for SIMD or just Parts

impl<'a, O: PackOrder> Iterator for PackSpan<'a, O> {
    type Item = Part;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(value) = self.get(0) {
            self.inner.consume(1, self.order);
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
            self.inner.consume(1, self.order);
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
