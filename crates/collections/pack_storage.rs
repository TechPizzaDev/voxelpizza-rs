use crate::{
    pack_span::{PackAccess, PackAccessMut, PackSpan, PartOffset, PartSize},
    pack_vec::BitsPerValue,
};

pub trait PackStorage<P>: PackAccess<P> {
    fn as_slice(&self) -> &[P];

    #[inline]
    fn as_span(&self) -> PackSpan<&[P], Self::BPV> {
        let bpv = self.bpv();
        let (part_end, tail_end) = make_end_tail(bpv.values_per_part(), self.len());
        PackSpan::from_parts(&self.as_slice()[..part_end], 0, tail_end, bpv)
    }
}

pub trait PackStorageMut<P>: PackStorage<P> + PackAccessMut<P> {
    fn as_slice_mut(&mut self) -> &mut [P];

    #[inline]
    fn as_span_mut(&mut self) -> PackSpan<&mut [P], Self::BPV> {
        let bpv = self.bpv();
        let (part_end, tail_len) = make_end_tail(bpv.values_per_part(), self.len());
        PackSpan::from_parts(&mut self.as_slice_mut()[..part_end], 0, tail_len, bpv)
    }
}

#[inline(always)]
const fn make_end_tail(vpp: PartSize, len: usize) -> (usize, PartOffset) {
    let vpp = vpp.get() as usize;
    let part_end = len.div_ceil(vpp);
    (part_end, (len % vpp) as PartOffset)
}
