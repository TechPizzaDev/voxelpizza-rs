use std::{hint::assert_unchecked, num::NonZeroU8};

use num_traits::{Euclid, PrimInt};

pub type Part = u64;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PartOffset(NonZeroU8);

impl PartOffset {
    pub const MAX: Self = Self(NonZeroU8::new(64).unwrap());

    pub const fn new(value: usize) -> Option<Self> {
        if value <= Self::MAX.get() {
            Some(Self(NonZeroU8::new((value + 1) as u8).unwrap()))
        } else {
            None
        }
    }

    pub const fn get(self) -> usize {
        let val = self.0.get() - 1;
        unsafe {
            assert_unchecked(val < Self::MAX.0.get());
        }
        val as usize
    }
}
impl Default for PartOffset {
    fn default() -> Self {
        Self(NonZeroU8::new(1).unwrap())
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PartSize(NonZeroU8);

impl PartSize {
    pub const MAX: Self = Self(NonZeroU8::new(64).unwrap());

    pub const fn new(value: usize) -> Option<Self> {
        if value != 0 && value <= Self::MAX.get() {
            Some(Self(NonZeroU8::new(value as u8).unwrap()))
        } else {
            None
        }
    }

    pub const fn get(self) -> usize {
        let val = self.0.get();
        unsafe {
            assert_unchecked(val <= Self::MAX.0.get());
        }
        val as usize
    }

    #[inline(always)]
    pub fn value_mask<T: PrimInt>(self) -> Option<T> {
        if let Some(shift) = (size_of::<T>() * 8).checked_sub(self.get()) {
            Some(T::zero().not().unsigned_shr(shift as u32))
        } else {
            None
        }
    }

    #[inline(always)]
    pub const fn values_per_part<T>(self) -> Option<PartSize> {
        let size = (size_of::<T>() * 8) / self.get();
        PartSize::new(size)
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct PackIndex(u64);

impl PackIndex {
    pub const ZERO: PackIndex = Self(0);

    const START_BITS: u32 = Part::BITS.ilog2();
    const LEN_BITS: u32 = Part::BITS - Self::START_BITS;

    const START_MASK: u64 = (1 << Self::START_BITS) - 1;
    const LEN_MASK: u64 = Part::MAX >> Self::START_BITS;

    #[inline]
    pub fn from_range(start: PartOffset, len: usize) -> Option<Self> {
        let len = len as u64;
        if len <= Self::LEN_MASK {
            let lo = start.get() as u64;
            let hi = len << Self::START_BITS;
            Some(Self(hi | lo))
        } else {
            None
        }
    }

    #[inline]
    pub fn from_len(len: usize) -> Option<Self> {
        Self::from_range(Default::default(), len)
    }

    #[inline]
    pub fn start(self) -> PartOffset {
        let start = PartOffset::new((self.0 & Self::START_MASK) as usize);
        unsafe { start.unwrap_unchecked() }
    }

    pub fn len(self) -> u64 {
        self.0 >> Self::START_BITS
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PartKey {
    pub part: usize,

    pub val: PartOffset,

    pub bit: PartOffset,
}

impl PartKey {
    #[inline(always)]
    pub fn new(index: usize, value_bits: PartSize, values_per_part: PartSize) -> Option<Self> {
        let (part, rem) = index.div_rem_euclid(&values_per_part.get());
        let val = PartOffset::new(rem)?;
        let bit = PartOffset::new(rem * value_bits.get())?;
        Some(Self {
            part,
            val,
            bit,
        })
    }
}

#[inline(always)]
pub fn part_count_ceil(value_len: usize, values_per_part: PartSize) -> usize {
    usize::try_from(value_len.div_ceil(values_per_part.get() as usize)).unwrap()
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
pub fn get<P, E>(part: P, bit_index: usize, value_mask: E) -> E
where
    E: PrimInt,
    P: PrimInt,
{
    E::from(part.unsigned_shr(bit_index as u32) & P::from(value_mask).unwrap()).unwrap()
}

#[inline(always)]
pub fn set<P, E>(part: P, bit_index: usize, value: E, value_mask: E) -> P
where
    E: PrimInt,
    P: PrimInt,
{
    let clear_mask = P::from(value_mask).unwrap().unsigned_shl(bit_index as u32);
    let set_mask = P::from(value & value_mask)
        .unwrap()
        .unsigned_shl(bit_index as u32);
    (part & clear_mask.not()) | set_mask
}
