use num_traits::PrimInt;
use seq_macro::seq;

use super::{PartSize, value_mask, values_per_part};

#[inline]
pub fn pack<P, E>(dst: &mut [P], dst_offset: usize, src: &[E], bits_per_value: PartSize)
where
    E: PrimInt,
    P: PrimInt,
{
    seq!(V in 1..=12 {
        match bits_per_value.get() {
            #(
                V => pack_const::<_, _, V>(dst, dst_offset, src),
            )*
            _ => pack_var(dst, dst_offset, src, bits_per_value)
        }
    });
}

#[inline(never)]
fn pack_const<P, E, const V: u8>(dst: &mut [P], dst_offset: usize, src: &[E])
where
    E: PrimInt,
    P: PrimInt,
{
    pack_core(dst, dst_offset, src, PartSize::new(V).unwrap())
}

#[inline(never)]
fn pack_var<P, E>(dst: &mut [P], dst_offset: usize, src: &[E], bits_per_value: PartSize)
where
    E: PrimInt,
    P: PrimInt,
{
    pack_core(dst, dst_offset, src, bits_per_value)
}

#[inline(always)]
fn pack_core<P, E>(mut dst: &mut [P], dst_offset: usize, mut src: &[E], bits_per_value: PartSize)
where
    E: PrimInt,
    P: PrimInt,
{
    todo!()
}
