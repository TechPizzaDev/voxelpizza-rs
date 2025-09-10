use std::{marker::PhantomData, ops::Deref};

use num_traits::PrimInt;

use super::{BitsPerValue, get, value_mask};

pub struct PackIter<S, E, BPV: BitsPerValue> {
    parts: S,
    index: usize,
    end: usize,
    bpv: BPV,
    _marker: PhantomData<E>,
}

impl<S, E: PrimInt, BPV: BitsPerValue> PackIter<S, E, BPV> {
    pub fn from_slice(parts: S, start: usize, len: usize, bpv: BPV) -> Self {
        let end = start.checked_add(len).unwrap();
        Self {
            parts,
            index: start,
            end,
            bpv,
            _marker: PhantomData,
        }
    }
}

impl<S, P: PrimInt, E: PrimInt, BPV: BitsPerValue> Iterator for PackIter<S, E, BPV>
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
        let key = self.bpv.part_key(index);
        self.index = index + 1;

        let mask = value_mask(self.bpv.bits_per_value()).unwrap();
        let part = unsafe { *self.parts.get_unchecked(key.part_index) };
        Some(get(part, key.bit_index, mask))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.end.wrapping_sub(self.index);
        (size, Some(size))
    }
}

impl<S, P: PrimInt, E: PrimInt, BPV: BitsPerValue> ExactSizeIterator for PackIter<S, E, BPV> where
    S: Deref<Target = [P]>
{
}
