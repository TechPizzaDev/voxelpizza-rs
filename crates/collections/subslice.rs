
// TODO: replace all *Cut traits with single generic trait
pub trait OwnedSliceIndex<T: ?Sized> {
    type Output;

    fn get(&self, slice: &T) -> Option<Self::Output> {
        todo!()
    }
}

pub trait RangeCut<I>
where
    Self: Sized,
{
    type Output;

    /// Cuts out a view of given length `len`, beginning at `start`.
    fn cut_checked(self, start: I, len: I) -> Option<Self::Output>;

    unsafe fn cut_unchecked(self, start: I, len: I) -> Self::Output;

    /// Cuts out a view of given length `len`, beginning at `start`.
    #[inline]
    fn cut(self, start: I, len: I) -> Self::Output {
        self.cut_checked(start, len).expect("range out of bounds")
    }
}

pub trait MidCut<I>
where
    Self: Sized,
{
    type Output;

    /// Cuts out a view beginning at `start`.
    fn cut_at_checked(self, mid: I) -> Option<Self::Output>;

    unsafe fn cut_at_unchecked(self, mid: I) -> Self::Output;

    /// Cuts out a view beginning at `start`.
    #[inline]
    fn cut_at(self, mid: I) -> Self::Output {
        self.cut_at_checked(mid).expect("range out of bounds")
    }
}

pub trait SplitCut<I>
where
    Self: Sized,
{
    type Output;

    fn split_at_checked(self, mid: I) -> Option<(Self::Output, Self::Output)>;

    unsafe fn split_at_unchecked(self, mid: I) -> (Self::Output, Self::Output);

    #[inline]
    fn split_at(self, mid: I) -> (Self::Output, Self::Output) {
        self.split_at_checked(mid).expect("mid out of bounds")
    }
}

impl<'a, T> RangeCut<usize> for &'a [T] {
    type Output = Self;

    #[inline]
    fn cut_checked(self, start: usize, len: usize) -> Option<Self::Output> {
        let end = start + len;
        self.get(start..end)
    }

    #[inline]
    unsafe fn cut_unchecked(self, start: usize, len: usize) -> Self::Output {
        let end = start + len;
        unsafe { self.get_unchecked(start..end) }
    }

    #[inline]
    fn cut(self, start: usize, len: usize) -> Self::Output {
        let end = start + len;
        &self[start..end]
    }
}

impl<'a, T> RangeCut<usize> for &'a mut [T] {
    type Output = Self;

    #[inline]
    fn cut_checked(self, start: usize, len: usize) -> Option<Self> {
        let end = start + len;
        self.get_mut(start..end)
    }

    #[inline]
    unsafe fn cut_unchecked(self, start: usize, len: usize) -> Self {
        let end = start + len;
        unsafe { self.get_unchecked_mut(start..end) }
    }

    #[inline]
    fn cut(self, start: usize, len: usize) -> Self {
        let end = start + len;
        &mut self[start..end]
    }
}

impl<'a, T> MidCut<usize> for &'a [T] {
    type Output = Self;

    #[inline]
    fn cut_at_checked(self, mid: usize) -> Option<Self::Output> {
        self.get(mid..)
    }

    #[inline]
    unsafe fn cut_at_unchecked(self, mid: usize) -> Self::Output {
        unsafe { self.get_unchecked(mid..) }
    }

    #[inline]
    fn cut_at(self, mid: usize) -> Self::Output {
        &self[mid..]
    }
}

impl<'a, T> MidCut<usize> for &'a mut [T] {
    type Output = Self;

    #[inline]
    fn cut_at_checked(self, mid: usize) -> Option<Self> {
        self.get_mut(mid..)
    }

    #[inline]
    unsafe fn cut_at_unchecked(self, mid: usize) -> Self {
        unsafe { self.get_unchecked_mut(mid..) }
    }

    #[inline]
    fn cut_at(self, mid: usize) -> Self {
        &mut self[mid..]
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
