use std::simd::{cmp::SimdPartialEq, LaneCount, Mask, Simd, SimdElement, SupportedLaneCount};

pub trait SliceSearch<T> {
    type Index;

    fn index_of_any_except<const N: usize>(&self, value: T) -> Option<Self::Index>
    where
        T: SimdElement + PartialEq,
        LaneCount<N>: SupportedLaneCount,
        Simd<T, N>: SimdPartialEq<Mask = Mask<T::Mask, N>>;
}

impl<T> SliceSearch<T> for [T] {
    type Index = usize;
    
    #[inline]
    fn index_of_any_except<const N: usize>(&self, value: T) -> Option<Self::Index>
    where
        T: SimdElement + PartialEq,
        LaneCount<N>: SupportedLaneCount,
        Simd<T, N>: SimdPartialEq<Mask = Mask<T::Mask, N>>,
    {
        let (prefix, suffix) = self.as_chunks::<N>();
        let broad = Simd::splat(value);
        let first = prefix
            .iter()
            .position(|v| Simd::from_array(*v).simd_ne(broad).any());

        if let Some(found) = first {
            let not_equals = Simd::from_array(prefix[found]).simd_ne(broad).to_bitmask() as u32;
            let vec_index = not_equals.trailing_zeros();
            let offset = found * N;
            return Some(offset + vec_index as usize);
        }

        if let Some(found) = suffix.iter().position(|v| *v != value) {
            let offset = prefix.len() * N;
            return Some(offset + found);
        }

        return None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T_N: usize = 4;

    fn test<const N: usize>(len: usize)
    where
        LaneCount<N>: SupportedLaneCount,
    {
        for i in 0..=len {
            let mut v = Vec::new();
            for j in 0..len {
                if i > j {
                    v.push(1);
                } else {
                    v.push(0);
                }
            }
            assert_eq!(v.index_of_any_except::<N>(1).unwrap_or(len), i);
        }
    }

    #[test]
    fn prefix() {
        for len in 0..T_N {
            test::<T_N>(len);
        }

        let a = 1;
        let b = 2;
        assert_eq!(vec![a; T_N - 1].index_of_any_except::<T_N>(a), None);
        assert_eq!(vec![a; T_N - 1].index_of_any_except::<T_N>(b), Some(0));
        assert_eq!(vec![a, b].index_of_any_except::<T_N>(a), Some(1));
    }

    #[test]
    fn aligned() {
        for i in 0..=T_N {
            test::<T_N>(i * T_N);
        }
    }

    #[test]
    fn suffix() {
        for len in T_N..(T_N * 2 - 1) {
            test::<T_N>(len);
        }

        let a = 1;
        let b = 2;
        assert_eq!(vec![a; T_N + 1].index_of_any_except::<T_N>(a), None);
        assert_eq!(vec![a; T_N + 1].index_of_any_except::<T_N>(b), Some(0));

        let mut vec = Vec::new();
        for _i in 0..T_N {
            vec.push(a);
        }
        vec.push(b);
        assert_eq!(vec.index_of_any_except::<T_N>(a), Some(T_N));
    }
}
