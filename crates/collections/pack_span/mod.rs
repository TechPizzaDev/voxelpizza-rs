mod pack_op;
pub use pack_op::pack;

mod unpack_op;
pub use unpack_op::unpack;

mod iter;
pub use iter::PackIter;

use std::{
    fmt,
    num::NonZeroU8,
    ops::{Deref, DerefMut, Range},
};

use num_traits::PrimInt;

use crate::{MidCut, RangeCut, SplitCut, pack_vec::BitsPerValue};

pub type PartOffset = u8;
pub type PartSize = NonZeroU8;

#[derive(Clone, Copy, Debug)]
pub struct PartKey {
    pub part_index: usize,

    pub bit_index: u32,
}

pub struct PackSpan<S, BPV: BitsPerValue> {
    parts: S,

    /// Inclusive offset into the first part.
    head_len: PartOffset,

    /// Exclusive offset into the last part.
    tail_len: PartOffset,

    bpv: BPV,
}

pub trait PackAccess<P> {
    type BPV: BitsPerValue;

    fn bpv(&self) -> Self::BPV;

    fn len(&self) -> usize;

    #[inline]
    fn part_len(&self) -> usize {
        part_count_ceil(self.len(), self.bpv().values_per_part())
    }

    fn get<E>(&self, index: usize) -> Option<E>
    where
        E: PrimInt,
        P: PrimInt;

    /*
    fn get<I>(&self, index: I) -> Option<I::Output>
    where
        I: SizedSliceIndex<Self>,
    {
        index.get(self)
    }
    */

    fn copy_to<E>(&self, dst: &mut impl PackAccessMut<E>)
    where
        E: PrimInt,
        P: PrimInt,
    {
        // TODO: optimize with pack/unpack
        self.cast_copy_to::<E, E>(dst);
    }

    fn cast_copy_to<E, T>(&self, dst: &mut impl PackAccessMut<T>)
    where
        E: PrimInt,
        T: PrimInt,
        P: PrimInt,
    {
        // TODO: optimize with pack/unpack

        // TODO: slot iterator that does get+set
        for index in 0..self.len() {
            let value: E = self.get(index).unwrap();
            dst.set(index, value).unwrap();
        }
    }
}

pub trait PackAccessMut<P>: PackAccess<P> {
    fn set<E>(&mut self, index: usize, value: E) -> Option<E>
    where
        E: PrimInt,
        P: PrimInt;

    fn fill<E>(&mut self, value: E)
    where
        E: PrimInt,
        P: PrimInt,
    {
        // TODO: optimize by writing full parts directly
        for i in 0..self.len() {
            self.set(i, value).unwrap();
        }
    }
}

impl PartKey {
    #[inline(always)]
    pub fn new(index: usize, bits_per_value: PartSize, values_per_part: PartSize) -> Self {
        let vpp = values_per_part.get() as usize;
        let (part_index, val_index) = (index / vpp, index % vpp);
        let bit_index = val_index as u32 * bits_per_value.get() as u32;
        Self {
            part_index,
            bit_index,
        }
    }
}

impl<S, P, BPV: BitsPerValue> PackSpan<S, BPV>
where
    S: Deref<Target = [P]>,
{
    #[inline]
    pub unsafe fn from_parts_unchecked(
        parts: S,
        head_len: PartOffset,
        tail_len: PartOffset,
        bpv: BPV,
    ) -> Self {
        debug_assert!(bpv.bits_per_part() <= size_of::<P>() * 8);
        debug_assert!(
            value_count(parts.len(), head_len, tail_len, bpv.values_per_part()).is_some()
        );
        Self {
            parts,
            head_len,
            tail_len,
            bpv,
        }
    }

    #[inline]
    pub fn from_parts(parts: S, head_len: PartOffset, tail_len: PartOffset, bpv: BPV) -> Self {
        assert!(bpv.bits_per_part() <= size_of::<P>() * 8);
        assert!(value_count(parts.len(), head_len, tail_len, bpv.values_per_part()).is_some());
        Self {
            parts,
            head_len,
            tail_len,
            bpv,
        }
    }

    #[inline]
    fn part_key(&self, index: usize) -> PartKey {
        self.bpv.part_key(self.head_len as usize + index)
    }

    #[inline(always)]
    fn is_valid_cut_range(&self, start: usize, len: usize) -> bool {
        if let Some(dst_end) = start.checked_add(len) {
            let src_len = self.len();
            dst_end <= src_len
        } else {
            false
        }
    }

    #[inline]
    fn get_cut_range(&self, start: usize, len: usize) -> (PartOffset, PartOffset, Range<usize>) {
        debug_assert!(self.is_valid_cut_range(start, len));

        let vpp = self.bpv.values_per_part().get() as usize;
        let new_start = self.head_len as usize + start;

        let part_start = new_start / vpp;
        let head_len = (new_start % vpp) as PartOffset;

        let new_end = new_start + len;
        let part_end = new_end.div_ceil(vpp);
        let tail_len = (new_end % vpp) as PartOffset;

        let part_range = part_start..part_end;
        (head_len, tail_len, part_range)
    }

    pub fn iter<E: PrimInt>(&self) -> PackIter<&[P], P, E, BPV> {
        let start = self.head_len as usize;
        let end = start.checked_add(self.len()).unwrap();
        PackIter::from_slice(&self.parts, start, end, self.bpv)
    }

    pub fn into_iter<E: PrimInt>(self) -> PackIter<S, P, E, BPV> {
        let start = self.head_len as usize;
        let end = start.checked_add(self.len()).unwrap();
        PackIter::from_slice(self.parts, start, end, self.bpv)
    }
}

impl<'a, 'b, P, BPV: BitsPerValue> SplitCut<usize> for &'b PackSpan<&'a [P], BPV> {
    type Output = PackSpan<&'a [P], BPV>;

    #[inline]
    fn split_at_checked(self, mid: usize) -> Option<(Self::Output, Self::Output)> {
        let src_len = self.len();
        if mid > src_len {
            return None;
        }
        unsafe {
            let head = self.cut_unchecked(0, mid);
            let tail = self.cut_unchecked(mid, src_len - mid);
            Some((head, tail))
        }
    }

    #[inline]
    unsafe fn split_at_unchecked(self, mid: usize) -> (Self::Output, Self::Output) {
        let src_len = self.len();
        debug_assert!(mid <= src_len);
        unsafe {
            let head = self.cut_unchecked(0, mid);
            let tail = self.cut_unchecked(mid, src_len - mid);
            (head, tail)
        }
    }
}

impl<'a, 'b, P, BPV: BitsPerValue> MidCut<usize> for &'b PackSpan<&'a [P], BPV> {
    type Output = PackSpan<&'a [P], BPV>;

    #[inline]
    fn cut_at_checked(self, mid: usize) -> Option<Self::Output> {
        let len = self.len();
        if mid > len {
            return None;
        }
        Some(unsafe { self.cut_unchecked(mid, len - mid) })
    }

    #[inline]
    unsafe fn cut_at_unchecked(self, mid: usize) -> Self::Output {
        let len = self.len();
        debug_assert!(mid <= len, "mid out of bounds");
        unsafe { self.cut_unchecked(mid, len - mid) }
    }
}
impl<'a, 'b, P, BPV: BitsPerValue> MidCut<usize> for &'b mut PackSpan<&'a mut [P], BPV> {
    type Output = PackSpan<&'b mut [P], BPV>;

    #[inline]
    fn cut_at_checked(self, mid: usize) -> Option<Self::Output> {
        let len = self.len();
        if mid > len {
            return None;
        }
        Some(unsafe { self.cut_unchecked(mid, len - mid) })
    }

    #[inline]
    unsafe fn cut_at_unchecked(self, mid: usize) -> Self::Output {
        let len = self.len();
        debug_assert!(mid <= len, "mid out of bounds");
        unsafe { self.cut_unchecked(mid, len - mid) }
    }
}
impl<'a, P, BPV: BitsPerValue> MidCut<usize> for PackSpan<&'a mut [P], BPV> {
    type Output = Self;

    #[inline]
    fn cut_at_checked(self, mid: usize) -> Option<Self::Output> {
        let len = self.len();
        if mid > len {
            return None;
        }
        Some(unsafe { self.cut_unchecked(mid, len - mid) })
    }

    #[inline]
    unsafe fn cut_at_unchecked(self, mid: usize) -> Self::Output {
        let len = self.len();
        debug_assert!(mid <= len, "mid out of bounds");
        unsafe { self.cut_unchecked(mid, len - mid) }
    }
}

impl<'a, 'b, P, BPV: BitsPerValue> RangeCut<usize> for &'b PackSpan<&'a [P], BPV> {
    type Output = PackSpan<&'a [P], BPV>;

    #[inline]
    fn cut_checked(self, start: usize, len: usize) -> Option<Self::Output> {
        if self.is_valid_cut_range(start, len) {
            Some(unsafe { self.cut_unchecked(start, len) })
        } else {
            None
        }
    }

    #[inline]
    unsafe fn cut_unchecked(self, start: usize, len: usize) -> Self::Output {
        let (head_len, tail_len, part_range) = self.get_cut_range(start, len);
        unsafe {
            let parts = self.parts.get_unchecked(part_range);
            Self::Output::from_parts_unchecked(parts, head_len, tail_len, self.bpv)
        }
    }
}
impl<'a, 'b, P, BPV: BitsPerValue> RangeCut<usize> for &'b mut PackSpan<&'a mut [P], BPV> {
    type Output = PackSpan<&'b mut [P], BPV>;

    #[inline]
    fn cut_checked(self, start: usize, len: usize) -> Option<Self::Output> {
        if self.is_valid_cut_range(start, len) {
            Some(unsafe { self.cut_unchecked(start, len) })
        } else {
            None
        }
    }

    #[inline]
    unsafe fn cut_unchecked(self, start: usize, len: usize) -> Self::Output {
        let (head_len, tail_len, part_range) = self.get_cut_range(start, len);
        unsafe {
            let parts = self.parts.get_unchecked_mut(part_range);
            Self::Output::from_parts_unchecked(parts, head_len, tail_len, self.bpv)
        }
    }
}
impl<'a, P, BPV: BitsPerValue> RangeCut<usize> for PackSpan<&'a mut [P], BPV> {
    type Output = Self;

    #[inline]
    fn cut_checked(self, start: usize, len: usize) -> Option<Self::Output> {
        if self.is_valid_cut_range(start, len) {
            Some(unsafe { self.cut_unchecked(start, len) })
        } else {
            None
        }
    }

    #[inline]
    unsafe fn cut_unchecked(self, start: usize, len: usize) -> Self::Output {
        let (head_len, tail_len, part_range) = self.get_cut_range(start, len);
        unsafe {
            let parts = self.parts.get_unchecked_mut(part_range);
            Self::Output::from_parts_unchecked(parts, head_len, tail_len, self.bpv)
        }
    }
}

// Cannot safely implement `SplitCut` for PackSpan over mut without tearing on shared slices.

impl<S, P, BPV: BitsPerValue> PackAccess<P> for PackSpan<S, BPV>
where
    S: Deref<Target = [P]>,
{
    type BPV = BPV;

    #[inline]
    fn bpv(&self) -> Self::BPV {
        self.bpv
    }

    #[inline]
    fn len(&self) -> usize {
        let value_count = value_count(
            self.parts.len(),
            self.head_len,
            self.tail_len,
            self.bpv.values_per_part(),
        );
        // SAFETY: assume Self is valid
        unsafe { value_count.unwrap_unchecked() }
    }

    #[inline]
    fn part_len(&self) -> usize {
        self.parts.len()
    }

    #[inline]
    fn get<E>(&self, index: usize) -> Option<E>
    where
        E: PrimInt,
        P: PrimInt,
    {
        let key = self.part_key(index);
        let mask = value_mask(self.bpv.bits_per_value()).unwrap();
        let part = self.parts.get(key.part_index)?;
        Some(get(*part, key.bit_index, mask))
    }
}

impl<S, P, BPV: BitsPerValue> PackAccessMut<P> for PackSpan<S, BPV>
where
    S: DerefMut<Target = [P]>,
{
    #[inline]
    fn set<E>(&mut self, index: usize, value: E) -> Option<E>
    where
        E: PrimInt,
        P: PrimInt,
    {
        let key = self.part_key(index);
        let mask = value_mask(self.bpv.bits_per_value()).unwrap();
        let part = self.parts.get_mut(key.part_index)?;
        let old_value = get(*part, key.bit_index, mask);
        *part = set(*part, key.bit_index, value, mask);
        Some(old_value)
    }

    fn fill<E>(&mut self, value: E)
    where
        E: PrimInt,
        P: PrimInt,
    {
        // TODO: optimize by writing full parts directly
        for i in 0..self.len() {
            // SAFETY: index is always in bounds
            unsafe { self.set(i, value).unwrap_unchecked() };
        }
    }
}

impl<S, P: PrimInt + fmt::Debug, BPV: BitsPerValue> fmt::Debug for PackSpan<S, BPV>
where
    S: Deref<Target = [P]>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter::<P>()).finish()
    }
}

#[inline(always)]
pub fn value_mask<E: PrimInt>(bits_per_value: PartSize) -> Option<E> {
    if let Some(shift) = (size_of::<E>() as u32 * 8).checked_sub(bits_per_value.get() as u32) {
        Some(E::zero().not().unsigned_shr(shift))
    } else {
        None
    }
}

#[inline(always)]
pub const fn values_per_part<P>(bits_per_value: PartSize) -> Option<PartSize> {
    let size = (size_of::<P>() * 8) as u32 / bits_per_value.get() as u32;
    if size != 0 && size <= (u8::MAX as u32) {
        PartSize::new(size as u8)
    } else {
        None
    }
}

#[inline(always)]
pub const fn part_count_ceil(value_len: usize, values_per_part: PartSize) -> usize {
    let vpp = values_per_part.get();
    value_len.div_ceil(vpp as usize)
}

#[inline(always)]
pub const fn value_count(
    part_count: usize,
    head_len: PartOffset,
    tail_len: PartOffset,
    values_per_part: PartSize,
) -> Option<usize> {
    let vpp = values_per_part.get();
    if let Some(body_len) = part_count.checked_mul(vpp as usize) {
        body_len.checked_sub(head_len as usize + tail_len as usize)
    } else {
        None
    }
}

#[inline(always)]
pub fn parallel_mask<P, E>(value_mask: E) -> P
where
    E: PrimInt,
    P: PrimInt,
{
    let value_mask = P::from(value_mask).unwrap();
    let mut parallel_mask = P::zero();
    let count = size_of::<P>() / size_of::<E>();
    for _ in 0..count {
        parallel_mask = parallel_mask << (size_of::<E>() * 8);
        parallel_mask = parallel_mask | value_mask;
    }
    parallel_mask
}

#[inline(always)]
pub fn get<P, E>(part: P, bit_index: u32, value_mask: E) -> E
where
    E: PrimInt,
    P: PrimInt,
{
    E::from(part.unsigned_shr(bit_index) & P::from(value_mask).unwrap()).unwrap()
}

#[inline(always)]
pub fn set<P, E>(part: P, bit_index: u32, value: E, value_mask: E) -> P
where
    E: PrimInt,
    P: PrimInt,
{
    let clear_mask = P::from(value_mask).unwrap().unsigned_shl(bit_index);
    let set_mask = P::from(value & value_mask).unwrap().unsigned_shl(bit_index);
    (part & clear_mask.not()) | set_mask
}

#[cfg(test)]
mod tests {

    use crate::{PackStorage, PackStorageMut, PackVec, pack_vec::ConstVec};

    use super::*;

    #[test]
    pub fn span_cut() {
        let cvec = ConstVec::<u64, 8>::default();

        let mut vec = PackVec::<u64>::new_var(NonZeroU8::try_from(1).unwrap());
        vec.extend_with(64, 0);
        let len_truth = vec.len();

        let span = vec.as_span_mut();
        let len0 = span.len();

        let cut1 = span.cut(4, 60);
        assert_eq!(cut1.len(), 60);

        let mut cut2 = cut1.cut(4, 56);
        assert_eq!(cut2.len(), 56);

        for i in 0..5 {
            cut2.set(i, 1).unwrap();
            assert_eq!(cut2.get(i), Some(1));
        }

        vec.set(0, 1).unwrap();

        println!("{:#b}", vec.get::<u64>(0).unwrap());
        println!("{:#b}", vec.get::<u64>(7).unwrap());
        println!("{:#b}", vec.get::<u64>(12).unwrap());
    }

    #[test]
    #[inline(never)]
    pub fn span_unpack() {
        let src = vec![u64::MAX; 4];
        let mut dst = vec![0u8; 64];

        for n in 1..8 {
            dst.as_mut_slice().fill(0);
            unpack(&mut dst, &src, 0, PartSize::new(n).unwrap());
            println!("{:?}", dst);
        }
    }

    #[test]
    #[inline(never)]
    pub fn span_iter() {
        let mut vec = PackVec::<u64>::new_var(NonZeroU8::try_from(2).unwrap());
        vec.extend_with(33, 1);
        vec.extend_with(33, 2);
        vec.extend_with(33, 3);

        println!("{:?}", vec);

        vec.iter::<u64>().eq(vec.as_span().iter());
    }
}
