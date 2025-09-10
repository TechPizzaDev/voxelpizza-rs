use std::{
    alloc::{Allocator, Global},
    fmt,
    marker::PhantomData,
};

use num_traits::PrimInt;
use raw_vec::RawVec;

use crate::pack_span::{
    self, PackAccess, PackAccessMut, PackIter, PackSpan, PartKey, PartOffset, PartSize,
    part_count_ceil, value_mask, values_per_part,
};
use crate::subslice::OwnedCut;

pub type ConstVec<P, const BPV: u8> = PackVec<P, ConstBPV<P, BPV>>;

/// Packed array of values. Each value consumes a specific amount of bits.
pub struct PackVec<P, BPV: BitsPerValue = VarBPV, A: Allocator = Global> {
    parts: RawVec<P, A>,
    len: usize,
    bpv: BPV,
}

pub trait BitsPerValue: Copy {
    fn bits_per_value(&self) -> PartSize;

    fn values_per_part(&self) -> PartSize;

    #[inline]
    fn bits_per_part(&self) -> usize {
        self.values_per_part().get() as usize * self.bits_per_value().get() as usize
    }

    #[inline]
    fn part_key(&self, value_index: usize) -> PartKey {
        PartKey::new(value_index, self.bits_per_value(), self.values_per_part())
    }
}

// TODO: print BitsPerValue::bits_per_part in Debug?
#[derive(Debug)]
pub struct VarBPV {
    bits_per_value: PartSize,
    values_per_part: PartSize,
}

#[derive(Debug)]
pub struct ConstBPV<P: 'static, const BPV: u8> {
    _marker: PhantomData<P>,
}

impl VarBPV {
    pub const fn new<P>(bits_per_value: PartSize) -> Self {
        Self {
            bits_per_value,
            values_per_part: values_per_part::<P>(bits_per_value).unwrap(),
        }
    }
}

impl Clone for VarBPV {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            bits_per_value: self.bits_per_value,
            values_per_part: self.values_per_part,
        }
    }
}
impl Copy for VarBPV {}
impl BitsPerValue for VarBPV {
    #[inline]
    fn bits_per_value(&self) -> PartSize {
        self.bits_per_value
    }

    #[inline]
    fn values_per_part(&self) -> PartSize {
        self.values_per_part
    }
}

impl<P, const BPV: u8> ConstBPV<P, BPV> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}
impl<P, const BPV: u8> Clone for ConstBPV<P, BPV> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            _marker: self._marker,
        }
    }
}
impl<P, const BPV: u8> Copy for ConstBPV<P, BPV> {}
impl<P, const BPV: u8> Default for ConstBPV<P, BPV> {
    #[inline]
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}
impl<P, const BPV: u8> BitsPerValue for ConstBPV<P, BPV> {
    #[inline]
    fn bits_per_value(&self) -> PartSize {
        PartSize::new(BPV).unwrap()
    }

    #[inline]
    fn values_per_part(&self) -> PartSize {
        values_per_part::<P>(self.bits_per_value()).unwrap()
    }
}

//#[derive(Debug)]
//pub struct PackedVecSlot {
//    part_idx: usize,
//    bit_idx: u32,
//    bits_per_value: BitSize,
//}

// TODO: resize in-place, changing BPV (maybe even generically, not specifically PackVec)

impl<P> PackVec<P, VarBPV> {
    #[inline]
    pub const fn new_var(bits_per_value: PartSize) -> Self {
        Self::new(VarBPV::new::<P>(bits_per_value))
    }
}
impl<P, BPV: BitsPerValue> PackVec<P, BPV, Global> {
    #[inline]
    pub const fn new(bpv: BPV) -> Self {
        Self::new_in(bpv, Global)
    }

    #[inline]
    pub fn with_capacity(capacity: usize, bpv: BPV) -> Self {
        Self::with_capacity_in(capacity, bpv, Global)
    }
}

impl<P, BPV: BitsPerValue, A: Allocator> PackVec<P, BPV, A> {
    #[inline]
    pub const fn new_in(bpv: BPV, alloc: A) -> Self {
        Self {
            parts: RawVec::new_in(alloc),
            len: 0,
            bpv,
        }
    }

    #[inline]
    pub fn with_capacity_in(capacity: usize, bpv: BPV, alloc: A) -> Self {
        let capacity = part_count_ceil(capacity, bpv.values_per_part());
        Self {
            parts: RawVec::with_capacity_in(capacity, alloc),
            len: 0,
            bpv,
        }
    }

    #[inline]
    pub const fn as_ptr(&self) -> *const P {
        self.parts.ptr()
    }

    #[inline]
    pub const fn as_mut_ptr(&mut self) -> *mut P {
        self.parts.ptr()
    }

    #[inline]
    pub const fn as_slice(&self) -> &[P] {
        unsafe { std::slice::from_raw_parts(self.as_ptr(), self.parts.capacity()) }
    }

    #[inline(always)]
    const fn make_end_tail(vpp: PartSize, len: usize) -> (usize, PartOffset) {
        let vpp = vpp.get() as usize;
        let part_end = len.div_ceil(vpp);
        (part_end, (len % vpp) as PartOffset)
    }

    #[inline]
    pub fn as_span(&self) -> PackSpan<&[P], BPV> {
        let bpv = self.bpv();
        let (part_end, tail_len) = Self::make_end_tail(bpv.values_per_part(), self.len());
        let parts = &self.as_slice()[..part_end];
        unsafe { PackSpan::from_parts_unchecked(parts, 0, tail_len, bpv) }
    }

    #[inline]
    pub const fn as_slice_mut(&mut self) -> &mut [P] {
        unsafe { std::slice::from_raw_parts_mut(self.as_mut_ptr(), self.parts.capacity()) }
    }

    #[inline]
    pub fn as_span_mut(&mut self) -> PackSpan<&mut [P], BPV> {
        let bpv = self.bpv();
        let (part_end, tail_len) = Self::make_end_tail(bpv.values_per_part(), self.len());
        let parts = &mut self.as_slice_mut()[..part_end];
        unsafe { PackSpan::from_parts_unchecked(parts, 0, tail_len, bpv) }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.parts.capacity() * self.bpv.values_per_part().get() as usize
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
            part_count_ceil(additional, self.bpv.values_per_part()),
        );
    }

    #[inline]
    pub fn push<E>(&mut self, value: E)
    where
        P: PrimInt,
        E: PrimInt,
    {
        let len = self.len;
        let key = self.bpv.part_key(len);
        if key.part_index == self.parts.capacity() {
            self.parts.grow_one();
        }

        let mask = value_mask(self.bpv.bits_per_value()).unwrap();
        unsafe {
            let part = self.as_mut_ptr().add(key.part_index);
            *part = pack_span::set(*part, key.bit_index, value, mask);
            self.set_len(len + 1);
        }
    }

    #[inline(never)]
    pub fn extend_with<E>(&mut self, n: usize, value: E)
    where
        P: PrimInt,
        E: PrimInt,
    {
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

    pub fn iter<E: PrimInt>(&self) -> PackIter<&[P], E, BPV> {
        self.as_span().into_iter()
    }

    #[inline]
    unsafe fn as_full_span_mut(&mut self) -> PackSpan<&mut [P], BPV> {
        let bpv = self.bpv;
        let tail_len = bpv.values_per_part().get();
        unsafe { PackSpan::from_parts_unchecked(self.as_slice_mut(), 0, tail_len, bpv) }
    }
}
impl<P, BPV: BitsPerValue + Default, A: Allocator + Default> Default for PackVec<P, BPV, A> {
    #[inline]
    fn default() -> Self {
        Self::new_in(BPV::default(), A::default())
    }
}

impl<P, BPV: BitsPerValue, A: Allocator> PackAccess<P> for PackVec<P, BPV, A> {
    type BPV = BPV;

    #[inline]
    fn bpv(&self) -> Self::BPV {
        self.bpv
    }

    #[inline]
    fn len(&self) -> usize {
        self.len
    }

    #[inline]
    fn get<E>(&self, index: usize) -> Option<E>
    where
        P: PrimInt,
        E: PrimInt,
    {
        if index >= self.len {
            return None;
        }
        let key = self.bpv.part_key(index);
        let mask = value_mask(self.bpv.bits_per_value()).unwrap();
        let part: P = unsafe { *self.as_ptr().add(key.part_index) };
        Some(pack_span::get(part, key.bit_index, mask))
    }
}
impl<P, BPV: BitsPerValue, A: Allocator> PackAccessMut<P> for PackVec<P, BPV, A> {
    #[inline]
    fn set<E>(&mut self, index: usize, value: E) -> Option<E>
    where
        P: PrimInt,
        E: PrimInt,
    {
        if index >= self.len {
            return None;
        }
        let key = self.bpv.part_key(index);
        let mask = value_mask(self.bpv.bits_per_value()).unwrap();
        let part: &mut P = unsafe { &mut *self.as_mut_ptr().add(key.part_index) };
        let old_value = pack_span::get(*part, key.bit_index, mask);
        *part = pack_span::set(*part, key.bit_index, value, mask);
        Some(old_value)
    }
}

impl<P: PrimInt + fmt::Debug, BPV: BitsPerValue, A: Allocator> fmt::Debug for PackVec<P, BPV, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_span().fmt(f)
    }
}
