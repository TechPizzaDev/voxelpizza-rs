use std::marker::PhantomData;

use crate::pack::part::{PartKey, PartSize};

pub trait PackOrder: Copy {
    fn value_bits(&self) -> PartSize;

    fn values_per_part(&self) -> PartSize;

    #[inline]
    fn bits_per_part(&self) -> usize {
        self.values_per_part().get() * self.value_bits().get()
    }

    #[inline]
    fn part_key(&self, value_index: usize) -> PartKey {
        PartKey::new(value_index, self.value_bits(), self.values_per_part())
    }
}

// TODO: print BitsPerValue::bits_per_part in Debug?
#[derive(Debug)]
pub struct VarPackOrder {
    value_bits: PartSize,
    values_per_part: PartSize,
}

#[derive(Debug)]
pub struct ConstPackOrder<P: 'static, const BPV: u8> {
    _marker: PhantomData<P>,
}

impl VarPackOrder {
    pub const fn new<P>(value_bits: PartSize) -> Self {
        Self {
            value_bits,
            values_per_part: value_bits.values_per_part::<P>().unwrap(),
        }
    }
}

impl Clone for VarPackOrder {
    #[inline]
    fn clone(&self) -> Self {
        Self { ..*self }
    }
}
impl Copy for VarPackOrder {}
impl PackOrder for VarPackOrder {
    #[inline]
    fn value_bits(&self) -> PartSize {
        self.value_bits
    }

    #[inline]
    fn values_per_part(&self) -> PartSize {
        self.values_per_part
    }
}

impl<P, const BPV: u8> ConstPackOrder<P, BPV> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}
impl<P, const BPV: u8> Clone for ConstPackOrder<P, BPV> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            _marker: self._marker,
        }
    }
}
impl<P, const BPV: u8> Copy for ConstPackOrder<P, BPV> {}
impl<P, const BPV: u8> Default for ConstPackOrder<P, BPV> {
    #[inline]
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}
impl<P, const BPV: u8> PackOrder for ConstPackOrder<P, BPV> {
    #[inline]
    fn value_bits(&self) -> PartSize {
        PartSize::new(BPV.into()).unwrap()
    }

    #[inline]
    fn values_per_part(&self) -> PartSize {
        self.value_bits().values_per_part::<P>().unwrap()
    }
}
