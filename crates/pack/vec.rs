use std::{
    alloc::{Allocator, Global},
    fmt,
};

use num_traits::PrimInt;
use raw_vec::RawVec;

use super::{
    order::{ConstPackOrder, PackOrder, VarPackOrder},
    part::{self, PackIndex, Part, PartSize, part_count_ceil},
    span::{PackAccess, PackAccessMut, PackSpan, PackSpanMut},
};
use collections::OwnedCut;

pub type ConstVec<T, const BPV: u8> = PackVec<ConstPackOrder<T, BPV>>;

/// Packed array of values. Each value consumes a specific amount of bits.
pub struct PackVec<O: PackOrder = VarPackOrder, A: Allocator = Global> {
    parts: RawVec<Part, A>,
    len: usize,
    order: O,
}

//#[derive(Debug)]
//pub struct PackedVecSlot {
//    part_idx: usize,
//    bit_idx: u32,
//    bits_per_value: BitSize,
//}

// TODO: resize in-place, changing BPV (maybe even generically, not specifically PackVec)

impl PackVec<VarPackOrder> {
    #[inline]
    pub const fn new_var(value_bits: PartSize) -> Self {
        Self::new(VarPackOrder::new::<Part>(value_bits))
    }
}
impl<O: PackOrder> PackVec<O, Global> {
    #[inline]
    pub const fn new(order: O) -> Self {
        Self::new_in(order, Global)
    }

    #[inline]
    pub fn with_capacity(capacity: usize, order: O) -> Self {
        Self::with_capacity_in(capacity, order, Global)
    }
}

impl<O: PackOrder, A: Allocator> PackVec<O, A> {
    #[inline]
    pub const fn new_in(order: O, alloc: A) -> Self {
        Self {
            parts: RawVec::new_in(alloc),
            len: 0,
            order,
        }
    }

    #[inline]
    pub fn with_capacity_in(capacity: usize, order: O, alloc: A) -> Self {
        let capacity = part_count_ceil(capacity, order.values_per_part());
        Self {
            parts: RawVec::with_capacity_in(capacity, alloc),
            len: 0,
            order,
        }
    }

    #[inline]
    pub const fn as_ptr(&self) -> *const Part {
        self.parts.ptr()
    }

    #[inline]
    pub const fn as_mut_ptr(&mut self) -> *mut Part {
        self.parts.ptr()
    }

    #[inline]
    pub const fn as_slice(&self) -> &[Part] {
        unsafe { std::slice::from_raw_parts(self.as_ptr(), self.parts.capacity()) }
    }

    /*
    #[inline(always)]
    const fn make_end_tail(vpp: PartSize, len: usize) -> (usize, PartOffset) {
        let vpp = vpp.get() as usize;
        let part_end = len.div_ceil(vpp);
        (part_end, (len % vpp) as PartOffset)
    }
    */

    #[inline]
    pub fn as_span(&self) -> PackSpan<'_, O> {
        let range = PackIndex::from_len(self.len).unwrap();
        unsafe { PackSpan::from_raw_parts(self.parts.non_null(), range, self.order) }
    }

    #[inline]
    pub const fn as_slice_mut(&mut self) -> &mut [Part] {
        unsafe { std::slice::from_raw_parts_mut(self.as_mut_ptr(), self.parts.capacity()) }
    }

    #[inline]
    pub fn as_span_mut(&mut self) -> PackSpanMut<'_, O> {
        let range = PackIndex::from_len(self.len).unwrap();
        unsafe { PackSpanMut::from_raw_parts(self.parts.non_null(), range, self.order) }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.parts.capacity() as usize * self.order.values_per_part().get() as usize
    }

    #[inline]
    pub unsafe fn set_len(&mut self, new_len: usize) {
        debug_assert!(new_len <= self.capacity());
        self.len = new_len;
    }

    #[inline(never)]
    pub fn reserve(&mut self, additional: usize) {
        self.parts.reserve(
            self.part_len(),
            part_count_ceil(additional, self.order.values_per_part()),
        );
    }

    #[inline]
    pub fn push<E: PrimInt>(&mut self, value: E) {
        let len = self.len;
        let key = self.order.part_key(len);
        if key.part_index == self.parts.capacity() {
            self.parts.grow_one();
        }

        let mask = self.order.value_bits().value_mask().unwrap();
        unsafe {
            let part = self.as_mut_ptr().add(key.part_index);
            *part = part::set(*part, key.bit_index, value, mask);
            self.set_len(len + 1);
        }
    }

    pub fn extend_with(&mut self, n: usize, value: Part) {
        let len = self.len;
        let new_len = len.strict_add(n);
        self.reserve(n);
        unsafe {
            self.as_full_span_mut()
                .cut_unchecked(len..new_len)
                .fill(value);
            self.set_len(new_len);
        }
    }

    #[inline]
    unsafe fn as_full_span_mut(&mut self) -> PackSpanMut<'_, O> {
        let range = PackIndex::from_len(self.capacity()).unwrap();
        unsafe { PackSpanMut::from_raw_parts(self.parts.non_null(), range, self.order) }
    }
}

impl<O: PackOrder + Default, A: Allocator + Default> Default for PackVec<O, A> {
    #[inline]
    fn default() -> Self {
        Self::new_in(O::default(), A::default())
    }
}

impl<O: PackOrder, A: Allocator> PackAccess for PackVec<O, A> {
    type Order = O;

    #[inline]
    fn order(&self) -> Self::Order {
        self.order
    }

    #[inline]
    fn len(&self) -> usize {
        self.len
    }

    #[inline]
    fn part_len(&self) -> usize {
        part_count_ceil(self.len, self.order.values_per_part())
    }

    #[inline]
    fn get<E: PrimInt>(&self, index: usize) -> Option<E> {
        if index >= self.len {
            return None;
        }
        let key = self.order.part_key(index);
        let mask = self.order.value_bits().value_mask().unwrap();
        let part: Part = unsafe { *self.as_ptr().add(key.part_index) };
        Some(part::get(part, key.bit_index, mask))
    }
}

impl<O: PackOrder, A: Allocator> PackAccessMut for PackVec<O, A> {
    #[inline]
    fn set<E: PrimInt>(&mut self, index: usize, value: E) -> Option<E> {
        if index >= self.len {
            return None;
        }
        let key = self.order.part_key(index);
        let mask = self.order.value_bits().value_mask().unwrap();
        let part: &mut Part = unsafe { &mut *self.as_mut_ptr().add(key.part_index) };
        let old_value = part::get(*part, key.bit_index, mask);
        *part = part::set(*part, key.bit_index, value, mask);
        Some(old_value)
    }
}

impl<O: PackOrder, A: Allocator> fmt::Debug for PackVec<O, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_span().fmt(f)
    }
}
