mod pack_op;
pub use pack_op::pack;

mod unpack_op;
pub use unpack_op::unpack;

mod iter;
pub use iter::PackIter;

use std::{
    fmt,
    hint::assert_unchecked,
    num::NonZeroU8,
    ops::{self, Bound, Deref, DerefMut, Range, RangeBounds},
    range,
};

use num_traits::PrimInt;

use crate::{
    OwnedCut, SplitCut,
    pack_vec::{PackOrder, VarPackOrder},
};

pub type PartOffset = u8;
pub type PartSize = NonZeroU8;

#[derive(Clone, Copy, Debug)]
pub struct PartKey {
    pub part_index: usize,

    pub bit_index: u32,
}

pub struct PackSpan<S, O: PackOrder = VarPackOrder> {
    parts: S,

    /// Inclusive offset into the first part.
    head_len: PartOffset,

    /// Exclusive offset into the last part.
    tail_len: PartOffset,

    order: O,
}

pub trait PackAccess<P> {
    type Order: PackOrder;

    fn order(&self) -> Self::Order;

    fn len(&self) -> usize;

    #[inline]
    fn part_len(&self) -> usize {
        part_count_ceil(self.len(), self.order().values_per_part())
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

impl<P, S, O: PackOrder> PackSpan<S, O>
where
    S: Deref<Target = [P]>,
{
    #[inline]
    pub fn from_parts<'a>(
        parts: S,
        head_len: PartOffset,
        tail_len: PartOffset,
        order: O,
    ) -> Result<Self, ()> {
        if order.bits_per_part() > size_of::<P>() * 8 {
            return Err(());
        }
        if value_count(parts.len(), head_len, tail_len, order.values_per_part()).is_none() {
            return Err(());
        }
        Ok(Self {
            parts,
            head_len,
            tail_len,
            order,
        })
    }

    #[inline]
    pub unsafe fn from_parts_unchecked(
        parts: S,
        head_len: PartOffset,
        tail_len: PartOffset,
        order: O,
    ) -> Self {
        Self {
            parts,
            head_len,
            tail_len,
            order,
        }
    }

    #[inline]
    fn part_key(&self, index: usize) -> PartKey {
        self.order
            .part_key(index.strict_add(self.head_len as usize))
    }

    #[inline]
    fn get_cut_range(
        &self,
        range: &impl RangeBounds<usize>,
    ) -> Result<(PartOffset, PartOffset, Range<usize>), ()> {
        let len = self.len();
        let start = start_bound(range);
        if start > len {
            return Err(());
        }

        let end = end_bound(range, len);
        if end > len {
            return Err(());
        }

        Ok(make_cut_range(
            start,
            end,
            self.head_len,
            self.order.values_per_part(),
        ))
    }

    pub fn iter<E: PrimInt>(&self) -> PackIter<&[P], E, O> {
        PackIter::from_slice(&self.parts, self.head_len as usize, self.len(), self.order)
    }

    pub fn into_iter<E: PrimInt>(self) -> PackIter<S, E, O> {
        let len = self.len();
        PackIter::from_slice(self.parts, self.head_len as usize, len, self.order)
    }
}

#[inline(always)]
fn start_bound(range: &impl RangeBounds<usize>) -> usize {
    match range.start_bound() {
        Bound::Included(i) => *i,
        Bound::Excluded(i) => *i,
        Bound::Unbounded => 0,
    }
}

#[inline(always)]
fn end_bound(range: &impl RangeBounds<usize>, len: usize) -> usize {
    match range.end_bound() {
        Bound::Included(i) => *i,
        Bound::Excluded(i) => *i,
        Bound::Unbounded => len,
    }
}

macro_rules! impl_owned_cut {
    ($index:ty) => {
        impl<P, S, O: PackOrder> OwnedCut<$index> for PackSpan<S, O>
        where
            S: Deref<Target = [P]> + OwnedCut<Range<usize>, Output = S>,
        {
            type Output = PackSpan<S, O>;

            #[inline]
            fn cut_checked(self, index: $index) -> Option<PackSpan<S, O>> {
                if let Ok((head_len, tail_len, part_range)) = self.get_cut_range(&index) {
                    let parts = unsafe { self.parts.cut_unchecked(part_range) };
                    Self::Output::from_parts(parts, head_len, tail_len, self.order).ok()
                } else {
                    None
                }
            }

            #[inline]
            unsafe fn cut_unchecked(self, index: $index) -> PackSpan<S, O> {
                let (head_len, tail_len, part_range) = make_cut_range(
                    start_bound(&index),
                    end_bound(&index, self.len()),
                    self.head_len,
                    self.order.values_per_part(),
                );
                unsafe {
                    let parts = self.parts.cut_unchecked(part_range);
                    Self::Output::from_parts_unchecked(parts, head_len, tail_len, self.order)
                }
            }
        }

        impl<'a, 'b, P, O: PackOrder> OwnedCut<$index> for &'b mut PackSpan<&'a mut [P], O> {
            type Output = PackSpan<&'b mut [P], O>;

            #[inline]
            fn cut_checked(self, index: $index) -> Option<Self::Output> {
                if let Ok((head_len, tail_len, part_range)) = self.get_cut_range(&index) {
                    let parts = unsafe { self.parts.cut_unchecked(part_range) };
                    Self::Output::from_parts(parts, head_len, tail_len, self.order).ok()
                } else {
                    None
                }
            }
        }
    };
}

impl_owned_cut!(ops::Range<usize>);
impl_owned_cut!(range::Range<usize>);
impl_owned_cut!(ops::RangeTo<usize>);
impl_owned_cut!(ops::RangeFrom<usize>);
impl_owned_cut!(range::RangeFrom<usize>);
impl_owned_cut!(ops::RangeInclusive<usize>);
impl_owned_cut!(range::RangeInclusive<usize>);
impl_owned_cut!(ops::RangeToInclusive<usize>);
impl_owned_cut!((ops::Bound<usize>, ops::Bound<usize>));

impl<'a, S, P, O: PackOrder> SplitCut<usize> for &'a PackSpan<S, O>
where
    S: Deref<Target = [P]>,
    Self: OwnedCut<Range<usize>, Output = PackSpan<S, O>>,
{
    type Output = PackSpan<S, O>;

    #[inline]
    fn split_at_checked(self, mid: usize) -> Option<(Self::Output, Self::Output)> {
        let src_len = self.len();
        if mid > src_len {
            return None;
        }
        unsafe {
            let head = self.cut_unchecked(0..mid);
            let tail = self.cut_unchecked(mid..src_len);
            Some((head, tail))
        }
    }
}

// Cannot safely implement `SplitCut` for PackSpan over mut without tearing on shared slices.

impl<S, P, O: PackOrder> PackAccess<P> for PackSpan<S, O>
where
    S: Deref<Target = [P]>,
{
    type Order = O;

    #[inline]
    fn order(&self) -> Self::Order {
        self.order
    }

    #[inline]
    fn len(&self) -> usize {
        let value_count = value_count(
            self.parts.len(),
            self.head_len,
            self.tail_len,
            self.order.values_per_part(),
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
        let mask = value_mask(self.order.bits_per_value()).unwrap();
        let part = self.parts.get(key.part_index)?;
        Some(get(*part, key.bit_index, mask))
    }
}

impl<S, P, O: PackOrder> PackAccessMut<P> for PackSpan<S, O>
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
        let mask = value_mask(self.order.bits_per_value()).unwrap();
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

impl<S, P: PrimInt + fmt::Debug, O: PackOrder> fmt::Debug for PackSpan<S, O>
where
    S: Deref<Target = [P]>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter::<P>()).finish()
    }
}

#[inline(always)]
pub fn value_mask<T: PrimInt>(bits_per_value: PartSize) -> Option<T> {
    if let Some(shift) = (size_of::<T>() as u32 * 8).checked_sub(bits_per_value.get() as u32) {
        Some(T::zero().not().unsigned_shr(shift))
    } else {
        None
    }
}

#[inline(always)]
pub const fn values_per_part<T>(bits_per_value: PartSize) -> Option<PartSize> {
    let size = (size_of::<T>() * 8) as u32 / bits_per_value.get() as u32;
    if size != 0 && size <= (u8::MAX as u32) {
        PartSize::new(size as u8)
    } else {
        None
    }
}

#[inline(always)]
pub const fn part_count_ceil(value_len: usize, values_per_part: PartSize) -> usize {
    value_len.div_ceil(values_per_part.get() as usize)
}

#[inline(always)]
pub const fn value_count(
    parts_len: usize,
    head_len: PartOffset,
    tail_len: PartOffset,
    values_per_part: PartSize,
) -> Option<usize> {
    if let Some(body_len) = parts_len.checked_mul(values_per_part.get() as usize) {
        body_len.checked_sub(head_len as usize + tail_len as usize)
    } else {
        None
    }
}

#[inline]
const fn make_cut_range(
    start: usize,
    end: usize,
    head_len: PartOffset,
    values_per_part: PartSize,
) -> (PartOffset, PartOffset, Range<usize>) {
    let start = start.wrapping_add(head_len as usize);
    let end = end.wrapping_add(head_len as usize);
    let vpp = values_per_part.get() as usize;

    let part_start = start / vpp;
    let head_len = (start % vpp) as PartOffset;

    let part_end = end.div_ceil(vpp);
    let tail_len = part_end.wrapping_mul(vpp).wrapping_sub(end) as PartOffset;

    (head_len, tail_len, part_start..part_end)
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
    unsafe {
        assert_unchecked(bit_index < (size_of::<P>() * 8) as u32);
    }
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
    use crate::{PackVec, pack_vec::ConstVec};

    use super::*;

    #[test]
    pub fn span_cut() {
        let mut cvec = ConstVec::<u64, 8>::default();
        cvec.extend_with(16, 3);

        let mut vec = PackVec::<u64>::new_var(NonZeroU8::try_from(1).unwrap());
        vec.extend_with(64, 0);

        let vec_len = vec.len();
        let span = vec.as_span_mut();
        assert_eq!(vec_len, span.len());

        let cut1 = span.cut(4..64);
        assert_eq!(cut1.len(), 60);

        let mut cut2 = cut1.cut(4..60);
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
