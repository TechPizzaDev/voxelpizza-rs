use num_traits::PrimInt;
use seq_macro::seq;

use super::part::PartSize;

#[inline]
pub fn unpack<P, E>(dst: &mut [E], src: &[P], src_offset: usize, value_bits: PartSize)
where
    E: PrimInt,
    P: PrimInt,
{
    seq!(V in 1..=12 {
        match value_bits.get() {
            #(
                V => unpack_const::<_, _, V>(dst, src, src_offset),
            )*
            _ => unpack_var(dst, src, src_offset, value_bits)
        }
    });
}

#[inline(never)]
fn unpack_const<P, E, const V: u8>(dst: &mut [E], src: &[P], src_offset: usize)
where
    E: PrimInt,
    P: PrimInt,
{
    unpack_core(dst, src, src_offset, PartSize::new(V.into()).unwrap())
}

#[inline(never)]
fn unpack_var<P, E>(dst: &mut [E], src: &[P], src_offset: usize, value_bits: PartSize)
where
    E: PrimInt,
    P: PrimInt,
{
    unpack_core(dst, src, src_offset, value_bits)
}

#[inline(always)]
fn unpack_core<P, E>(mut dst: &mut [E], mut src: &[P], src_offset: usize, value_bits: PartSize)
where
    E: PrimInt,
    P: PrimInt,
{
    let values_per_part = value_bits.values_per_part::<P>().unwrap().get();
    let src_idx = src_offset / values_per_part;
    let src_rem = src_offset % values_per_part;

    // Widen mask here (E -> P), which helps LLVM to vectorize;
    // P will never contain bits outside E range, making unwraps no-op.
    let value_mask = P::from(value_bits.value_mask::<E>().unwrap()).unwrap();
    let value_bits = value_bits.get();

    if src_rem != 0 {
        let head_offset = src_rem * value_bits;
        let head_part = src[src_idx].unsigned_shr(head_offset as u32);
        src = &src[src_idx..];

        let head_count = (values_per_part - src_rem).min(dst.len());
        let head_dst;
        (head_dst, dst) = dst.split_at_mut(head_count);
        unpack_part(head_dst, head_part, value_bits, value_mask);
    }

    for (dst_chunk, src_part) in dst.chunks_mut(values_per_part).zip(src) {
        unpack_part(dst_chunk, *src_part, value_bits, value_mask);
    }
}

#[inline(always)]
fn unpack_part<P, E>(dst: &mut [E], part: P, value_bits: usize, value_mask: P)
where
    E: PrimInt,
    P: PrimInt,
{
    for i in 0..dst.len() {
        let bits = part.unsigned_shr((i * value_bits) as u32);
        dst[i] = E::from(bits & value_mask).unwrap();
    }
}
