use std::{
    fmt,
    marker::PhantomData,
    ops::{Bound, Range, RangeBounds},
    ptr::NonNull,
};

use num_traits::PrimInt;

use super::{
    part::{self, PackIndex, Part, PartKey, PartOffset, PartSize, part_count_ceil},
    order::{PackOrder, VarPackOrder},
};
use crate::{OwnedCut, SplitCut};

#[derive(Clone)]
pub(super) struct PackSpanInner {
    ptr: NonNull<Part>,
    range: PackIndex,
}

impl PackSpanInner {
    #[inline]
    fn len(&self) -> usize {
        usize::try_from(self.range.len()).unwrap()
    }

    #[inline]
    fn bit_len(&self, bits_per_value: PartSize) -> u64 {
        self.range.len() * (bits_per_value.get() as u64)
    }

    #[inline]
    unsafe fn with_bounds(
        &self,
        start: Bound<&usize>,
        end: Bound<&usize>,
        values_per_part: PartSize,
    ) -> Result<Self, ()> {
        let len = self.len();
        let start = match start {
            Bound::Included(i) => *i,
            Bound::Excluded(i) => *i + 1,
            Bound::Unbounded => 0,
        };
        if start > len {
            return Err(());
        }

        let end = match end {
            Bound::Included(i) => *i + 1,
            Bound::Excluded(i) => *i,
            Bound::Unbounded => len,
        };
        if end > len {
            return Err(());
        }

        let val_start = start + self.range.start().get();
        let vpp = values_per_part.get();
        let (new_offset, new_start) = (val_start / vpp, val_start % vpp);
        let new_len = end - start;

        Ok(Self {
            ptr: unsafe { self.ptr.add(new_offset) },
            range: PackIndex::from_range(PartOffset::new(new_start).unwrap(), new_len).unwrap(),
        })
    }

    #[inline]
    pub(super) fn consume(&mut self, amount: usize, values_per_part: PartSize) {
        unsafe {
            *self = self
                .with_bounds(Bound::Included(&amount), Bound::Unbounded, values_per_part)
                .unwrap();
        }
    }
}

#[derive(Clone)]
pub struct PackSpan<'a, O: PackOrder = VarPackOrder> {
    pub(super) inner: PackSpanInner,
    pub(super) order: O,
    _ty: PhantomData<&'a [Part]>,
}

pub struct PackSpanMut<'a, O: PackOrder = VarPackOrder> {
    pub(super) inner: PackSpanInner,
    pub(super) order: O,
    _ty: PhantomData<&'a mut [Part]>,
}

pub trait PackAccess {
    type Order: PackOrder;

    fn order(&self) -> Self::Order;

    fn len(&self) -> usize;

    #[inline]
    fn part_len(&self) -> usize {
        part_count_ceil(self.len(), self.order().values_per_part())
    }

    fn get<E: PrimInt>(&self, index: usize) -> Option<E>;

    /*
    fn get<I>(&self, index: I) -> Option<I::Output>
    where
        I: SizedSliceIndex<Self>,
    {
        index.get(self)
    }
    */

    fn copy_to(&self, dst: &mut impl PackAccessMut) {
        // TODO: optimize with pack/unpack

        // TODO: slot iterator that does get+set
        for index in 0..self.len() {
            let value: Part = self.get(index).unwrap();
            dst.set(index, value).unwrap();
        }

        /*
        match bpv.bits_per_value().get() {
            ..=08 => src_span.copy_to::<u8>(dst_span),
            ..=16 => src_span.copy_to::<u16>(dst_span),
            ..=32 => src_span.copy_to::<u32>(dst_span),
            bpv => panic_unsupported_bpv(bpv),
        };
        */
    }
}

pub trait PackAccessMut: PackAccess {
    fn set<E>(&mut self, index: usize, value: E) -> Option<E>
    where
        E: PrimInt;

    fn fill<E>(&mut self, value: E)
    where
        E: PrimInt,
    {
        // TODO: optimize by writing full parts directly
        for i in 0..self.len() {
            self.set(i, value).unwrap();
        }
    }
}

impl<'a, O: PackOrder> From<PackSpanMut<'a, O>> for PackSpan<'a, O> {
    #[inline]
    fn from(value: PackSpanMut<'a, O>) -> Self {
        Self {
            inner: value.inner,
            order: value.order,
            _ty: PhantomData,
        }
    }
}

impl<'a, O: PackOrder> PackSpanMut<'a, O> {
    #[inline]
    pub fn from_slice_mut(parts: &'a mut [Part], range: PackIndex, order: O) -> Result<Self, ()> {
        if order.bits_per_part() > size_of::<Part>() * 8 {
            return Err(());
        }
        if range.len() > (parts.len() * order.values_per_part().get()) as u64 {
            return Err(());
        }
        Ok(unsafe {
            let ptr = NonNull::new_unchecked(parts.as_mut_ptr());
            Self::from_raw_parts(ptr, range, order)
        })
    }

    #[inline]
    pub unsafe fn from_raw_parts(ptr: NonNull<Part>, range: PackIndex, order: O) -> Self {
        Self {
            inner: PackSpanInner { ptr, range },
            order,
            _ty: PhantomData,
        }
    }

    #[inline]
    pub fn bit_len(&self) -> u64 {
        self.inner.bit_len(self.order.value_bits())
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn iter<'b>(&'b self) -> PackSpan<'b, O> {
        PackSpan {
            inner: self.inner.clone(),
            order: self.order,
            _ty: PhantomData,
        }
    }

    #[inline]
    fn make_part(&self, index: usize) -> Option<PartKey> {
        match index.checked_add(self.inner.range.start().get()) {
            Some(index) => Some(PartKey::new(
                index,
                self.order.value_bits(),
                self.order.values_per_part(),
            )),
            None => None,
        }
    }
}

impl<'a, O: PackOrder> PackSpan<'a, O> {
    #[inline]
    pub fn from_slice(parts: &'a [Part], range: PackIndex, order: O) -> Result<Self, ()> {
        if order.bits_per_part() > size_of::<Part>() * 8 {
            return Err(());
        }
        if range.len() > (parts.len() * order.values_per_part().get() as usize) as u64 {
            return Err(());
        }
        Ok(unsafe {
            let ptr = NonNull::new_unchecked(parts.as_ptr() as *mut Part);
            Self::from_raw_parts(ptr, range, order)
        })
    }

    #[inline]
    pub unsafe fn from_raw_parts(ptr: NonNull<Part>, range: PackIndex, order: O) -> Self {
        Self {
            inner: PackSpanInner { ptr, range },
            order,
            _ty: PhantomData,
        }
    }

    #[inline]
    pub fn bit_len(&self) -> u64 {
        self.inner.bit_len(self.order.value_bits())
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn iter<'b>(&'b self) -> PackSpan<'b, O> {
        self.clone()
    }

    #[inline]
    fn make_part(&self, index: usize) -> Option<PartKey> {
        match index.checked_add(self.inner.range.start().get()) {
            Some(index) => Some(PartKey::new(
                index,
                self.order.value_bits(),
                self.order.values_per_part(),
            )),
            None => None,
        }
    }
}

impl<'a, I: RangeBounds<usize>, O: PackOrder> OwnedCut<I> for &'a PackSpan<'a, O> {
    type Output = PackSpan<'a, O>;

    #[inline]
    fn cut_checked(self, index: I) -> Option<Self::Output> {
        match unsafe {
            self.inner.with_bounds(
                index.start_bound(),
                index.end_bound(),
                self.order.values_per_part(),
            )
        } {
            Ok(inner) => Some(Self::Output { inner, ..*self }),
            Err(_) => None,
        }
    }
}

impl<'a, I: RangeBounds<usize>, O: PackOrder> OwnedCut<I> for PackSpanMut<'a, O> {
    type Output = PackSpanMut<'a, O>;

    #[inline]
    fn cut_checked(self, index: I) -> Option<Self::Output> {
        match unsafe {
            self.inner.with_bounds(
                index.start_bound(),
                index.end_bound(),
                self.order.values_per_part(),
            )
        } {
            Ok(inner) => Some(Self::Output { inner, ..self }),
            Err(_) => None,
        }
    }
}

impl<'a, 'b, I: RangeBounds<usize>, O: PackOrder> OwnedCut<I> for &'b mut PackSpanMut<'a, O> {
    type Output = PackSpanMut<'b, O>;

    #[inline]
    fn cut_checked(self, index: I) -> Option<Self::Output> {
        match unsafe {
            self.inner.with_bounds(
                index.start_bound(),
                index.end_bound(),
                self.order.values_per_part(),
            )
        } {
            Ok(inner) => Some(Self::Output { inner, ..*self }),
            Err(_) => None,
        }
    }
}

impl<'a, 'b, O: PackOrder> SplitCut<usize> for &'a PackSpan<'b, O>
where
    Self: OwnedCut<Range<usize>, Output = PackSpan<'b, O>>,
{
    type Output = PackSpan<'a, O>;

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

impl<'a, O: PackOrder> PackAccess for PackSpan<'a, O> {
    type Order = O;

    #[inline]
    fn order(&self) -> Self::Order {
        self.order
    }

    #[inline]
    fn len(&self) -> usize {
        self.len()
    }

    #[inline]
    fn get<T>(&self, index: usize) -> Option<T>
    where
        T: PrimInt,
    {
        let key = self.make_part(index)?;
        let mask = self.order.value_bits().value_mask().unwrap();
        let part = unsafe { self.inner.ptr.add(key.part_index).read() };
        Some(part::get(part, key.bit_index as u32, mask))
    }
}

impl<'a, O: PackOrder> PackAccess for PackSpanMut<'a, O> {
    type Order = O;

    #[inline]
    fn order(&self) -> Self::Order {
        self.order
    }

    #[inline]
    fn len(&self) -> usize {
        self.len()
    }

    #[inline]
    fn get<E: PrimInt>(&self, index: usize) -> Option<E> {
        let key = self.make_part(index)?;
        let mask = self.order.value_bits().value_mask().unwrap();
        let part = unsafe { self.inner.ptr.add(key.part_index).read() };
        Some(part::get(part, key.bit_index as u32, mask))
    }
}

impl<'a, O: PackOrder> PackAccessMut for PackSpanMut<'a, O> {
    #[inline]
    fn set<E: PrimInt>(&mut self, index: usize, value: E) -> Option<E> {
        let key = self.make_part(index)?;
        let mask = self.order.value_bits().value_mask().unwrap();
        let part = unsafe { self.inner.ptr.add(key.part_index).as_mut() };
        let old_value = part::get(*part, key.bit_index, mask);
        *part = part::set(*part, key.bit_index, value, mask);
        Some(old_value)
    }

    fn fill<E: PrimInt>(&mut self, value: E) {
        // TODO: optimize by writing full parts directly
        for i in 0..PackAccess::len(self) {
            // SAFETY: index is always in bounds
            unsafe { self.set(i, value).unwrap_unchecked() };
        }
    }
}

impl<'a, O: PackOrder> fmt::Debug for PackSpan<'a, O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<'a, O: PackOrder> fmt::Debug for PackSpanMut<'a, O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    use crate::pack::vec::{ConstVec, PackVec};

    #[test]
    pub fn span_cut() {
        let mut cvec = ConstVec::<u64, 8>::default();
        cvec.extend_with(16, 3);

        let mut vec = PackVec::new_var(PartSize::new(1).unwrap());
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
            crate::pack::unpack(&mut dst, &src, 0, PartSize::new(n).unwrap());
            println!("{:?}", dst);
        }
    }

    #[test]
    #[inline(never)]
    pub fn span_iter() {
        let mut vec = PackVec::new_var(PartSize::new(2).unwrap());
        vec.extend_with(33, 1);
        vec.extend_with(33, 2);
        vec.extend_with(33, 3);

        println!("{:?}", vec);

        vec.as_span().eq(vec.as_span());
    }
}
