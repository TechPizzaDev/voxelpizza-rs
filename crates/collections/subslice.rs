use std::{ops, range};

pub trait OwnedCut<I>: Sized {
    type Output = Self;

    fn cut_checked(self, index: I) -> Option<Self::Output>;

    unsafe fn cut_unchecked(self, index: I) -> Self::Output {
        unsafe { self.cut_checked(index).unwrap_unchecked() }
    }

    /// Cuts out a view of given length `len`, beginning at `start`.
    #[inline]
    fn cut(self, index: I) -> Self::Output {
        self.cut_checked(index).expect("range out of bounds")
    }
}

macro_rules! impl_owned_cut {
    ($index:ty) => {
        impl<'a, T> OwnedCut<$index> for &'a [T] {
            fn cut_checked(self, index: $index) -> Option<Self::Output> {
                self.get(index)
            }

            unsafe fn cut_unchecked(self, index: $index) -> Self::Output {
                unsafe { self.get_unchecked(index) }
            }
        }

        impl<'a, T> OwnedCut<$index> for &'a mut [T] {
            fn cut_checked(self, index: $index) -> Option<Self::Output> {
                self.get_mut(index)
            }

            unsafe fn cut_unchecked(self, index: $index) -> Self::Output {
                unsafe { self.get_unchecked_mut(index) }
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

pub trait SplitCut<I>: Sized {
    type Output;

    fn split_at_checked(self, mid: I) -> Option<(Self::Output, Self::Output)>;

    #[inline]
    unsafe fn split_at_unchecked(self, mid: I) -> (Self::Output, Self::Output) {
        unsafe { self.split_at_checked(mid).unwrap_unchecked() }
    }

    #[inline]
    fn split_at(self, mid: I) -> (Self::Output, Self::Output) {
        self.split_at_checked(mid).expect("mid out of bounds")
    }
}

impl<'a, T> SplitCut<usize> for &'a [T] {
    type Output = Self;

    #[inline]
    fn split_at_checked(self, mid: usize) -> Option<(Self, Self)> {
        self.split_at_checked(mid)
    }

    #[inline]
    unsafe fn split_at_unchecked(self, mid: usize) -> (Self, Self) {
        unsafe { self.split_at_unchecked(mid) }
    }

    #[inline]
    fn split_at(self, mid: usize) -> (Self, Self) {
        self.split_at(mid)
    }
}

impl<'a, T> SplitCut<usize> for &'a mut [T] {
    type Output = Self;

    #[inline]
    fn split_at_checked(self, mid: usize) -> Option<(Self, Self)> {
        self.split_at_mut_checked(mid)
    }

    #[inline]
    unsafe fn split_at_unchecked(self, mid: usize) -> (Self, Self) {
        unsafe { self.split_at_mut_unchecked(mid) }
    }

    #[inline]
    fn split_at(self, mid: usize) -> (Self, Self) {
        self.split_at_mut(mid)
    }
}
