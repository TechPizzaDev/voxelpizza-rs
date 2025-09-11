use std::{marker::PhantomData, ops::Deref};

use num_traits::PrimInt;

use super::{PackOrder, get, value_mask};

pub struct PackIter<S, E, O: PackOrder> {
    parts: S,
    index: usize,
    end: usize,
    order: O,
    _marker: PhantomData<E>,
}

impl<S, E: PrimInt, O: PackOrder> PackIter<S, E, O> {
    pub fn from_slice(parts: S, start: usize, len: usize, order: O) -> Self {
        let end = start.checked_add(len).unwrap();
        Self {
            parts,
            index: start,
            end,
            order,
            _marker: PhantomData,
        }
    }
}

impl<S, P: PrimInt, E: PrimInt, O: PackOrder> Iterator for PackIter<S, E, O>
where
    S: Deref<Target = [P]>,
{
    type Item = E;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        if index >= self.end {
            return None;
        }
        let key = self.order.part_key(index);
        self.index = index + 1;

        let mask = value_mask(self.order.bits_per_value()).unwrap();
        let part = unsafe { *self.parts.get_unchecked(key.part_index) };
        Some(get(part, key.bit_index, mask))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.end.wrapping_sub(self.index);
        (size, Some(size))
    }
}

impl<S, P: PrimInt, E: PrimInt, O: PackOrder> ExactSizeIterator for PackIter<S, E, O> where
    S: Deref<Target = [P]>
{
}
