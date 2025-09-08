use num_traits::PrimInt;
use seq_macro::seq;

use super::{PartSize, value_mask, values_per_part};

#[inline]
pub fn unpack<P, E>(dst: &mut [E], src: &[P], src_offset: usize, bits_per_value: PartSize)
where
    E: PrimInt,
    P: PrimInt,
{
    seq!(V in 1..=12 {
        match bits_per_value.get() {
            #(
                V => unpack_const::<_, _, V>(dst, src, src_offset),
            )*
            _ => unpack_var(dst, src, src_offset, bits_per_value)
        }
    });
}

#[inline(never)]
fn unpack_const<P, E, const V: u8>(dst: &mut [E], src: &[P], src_offset: usize)
where
    E: PrimInt,
    P: PrimInt,
{
    unpack_core(dst, src, src_offset, PartSize::new(V).unwrap())
}

#[inline(never)]
fn unpack_var<P, E>(dst: &mut [E], src: &[P], src_offset: usize, bits_per_value: PartSize)
where
    E: PrimInt,
    P: PrimInt,
{
    unpack_core(dst, src, src_offset, bits_per_value)
}

#[inline(always)]
fn unpack_core<P, E>(mut dst: &mut [E], mut src: &[P], src_offset: usize, bits_per_value: PartSize)
where
    E: PrimInt,
    P: PrimInt,
{
    let values_per_part = values_per_part::<P>(bits_per_value).unwrap().get() as usize;
    let src_idx = src_offset / values_per_part;
    let src_rem = src_offset % values_per_part;

    let value_mask = value_mask::<E>(bits_per_value).unwrap();
    let bits_per_value = bits_per_value.get() as usize;

    if src_rem != 0 {
        let head_offset = src_rem * bits_per_value;
        let head_part = src[src_idx].unsigned_shr(head_offset as u32);
        src = &src[src_idx..];

        let head_count = (values_per_part - src_rem).min(dst.len());
        let head_dst;
        (head_dst, dst) = dst.split_at_mut(head_count);
        unpack_part(head_dst, head_part, bits_per_value, value_mask);
    }

    for (dst_chunk, src_part) in dst.chunks_mut(values_per_part).zip(src) {
        unpack_part(dst_chunk, *src_part, bits_per_value, value_mask);
    }
}

#[inline(always)]
fn unpack_part<P, E>(dst: &mut [E], part: P, bits_per_value: usize, value_mask: E)
where
    E: PrimInt,
    P: PrimInt,
{
    // Widen mask here (E -> P), which allows LLVM to vectorize; 
    // P will never contain bits outside E range, making unwraps no-op.
    let value_mask = P::from(value_mask).unwrap();
    
    for i in 0..dst.len() {
        let bits = part.unsigned_shr((i * bits_per_value) as u32);
        dst[i] = E::from(bits & value_mask).unwrap();
    }
}
