use std::{
    collections::hash_map::Entry,
    num::NonZeroUsize,
    simd::{LaneCount, Mask, Simd, SimdElement, SupportedLaneCount, cmp::SimdPartialEq},
};

use collections::{
    IndexMap, OwnedCut, PackVec,
    pack_span::{PackAccess, PackAccessMut, PackSpan, PartSize},
    pack_vec::{PackOrder, ConstPackOrder, ConstVec, VarPackOrder},
};
use iters::search::SliceSearch;
use num_traits::PrimInt;

use crate::block::{BlockCoord, BlockId, BlockSize};

use super::{BlockStorage, Chunk, get_index_base};

type Part = u64;
type PalIdx = u32;

#[derive(Debug)]
pub struct ChunkPalette {
    indices: IndexMap<BlockId, PalIdx>,
    data: PackVec<Part>,
}

const fn get_storage_bits_for_palette(count: usize) -> PartSize {
    let size = if count <= 1 {
        1
    } else {
        let max_bits = (size_of::<BlockId>() * 8) as u32;
        assert!(max_bits <= PartSize::MAX.get() as u32);

        let free_bits = (count - 1).leading_zeros();
        max_bits
            .checked_sub(free_bits)
            .expect("count exceeds representable range") as u8
    };
    PartSize::new(size).unwrap()
}

fn block_index_of_any_except<const N: usize>(slice: &[BlockId], value: BlockId) -> Option<usize>
where
    LaneCount<N>: SupportedLaneCount,
    Simd<u32, N>: SimdPartialEq<Mask = Mask<<u32 as SimdElement>::Mask, N>>,
{
    let slice_32 = bytemuck::cast_slice::<BlockId, u32>(slice);
    slice_32.index_of_any_except::<N>(value.0)
}

impl ChunkPalette {
    #[inline(never)]
    fn get_blocks_core<E, const BPV: u8, const N: usize>(
        &self,
        offset: BlockCoord,
        size: BlockSize,
        dst_offset: BlockCoord,
        dst_bounds: BlockSize,
        dst: &mut [BlockId],
    ) where
        E: 'static + SimdElement + PrimInt,
        LaneCount<N>: SupportedLaneCount,
        Simd<E, N>: SimdPartialEq<Mask = Mask<E::Mask, N>>,
    {
        let depth = self.depth().get();
        let width = self.width().get();
        let stride = size.width;

        // TODO: increase buffer size preferrably to layer-wide instead of row-wide, to increase SIMD utilization
        let mut index_buffer: ConstVec<E, BPV> = Default::default();
        index_buffer.extend_with(stride, 0);

        let span = &mut index_buffer.as_span_mut();

        for y in 0..size.height {
            let src_y = offset.y + y;
            let dst_y = dst_offset.y + y;

            for z in 0..size.depth {
                let dst_z = dst_offset.z + z;
                let dst_idx =
                    get_index_base(dst_bounds.depth, dst_bounds.width, dst_y, dst_z) + dst_offset.x;
                let dst_slice = dst.cut(dst_idx..(dst_idx + stride));

                let src_z = offset.z + z;
                let src_idx = offset.x + get_index_base(depth, width, src_y, src_z);
                self.get_contiguous_blocks(dst_slice, span.cut(src_idx..));
            }
        }
    }

    fn get_contiguous_blocks<E, const BPV: u8, const N: usize>(
        &self,
        mut dst: &mut [BlockId],
        index_buffer: PackSpan<&mut [E], ConstPackOrder<E, BPV>>,
    ) where
        E: 'static + SimdElement + PrimInt,
        LaneCount<N>: SupportedLaneCount,
        Simd<E, N>: SimdPartialEq<Mask = Mask<E::Mask, N>>,
    {
        assert_eq!(index_buffer.len(), dst.len());

        let storage = &self.data;
        let palette = &self.indices.list;

        let mut src = index_buffer;

        // TODO: Add BitArray.IndexOfAnyExcept to reduce unpacking?

        // Unpack block indices in bulk.
        storage.copy_to(&mut src);

        while src.len() > 0 {
            // TODO: assert that src (with a specific bits_per_value) can never return values larger than palette len;
            //       could remove boundcheck
            let index: E = src.get::<E>(0).unwrap();

            // Move ahead while there are duplicates in the source.
            let len = None; // TODO: src.index_of_any_except(index);
            let len = len.unwrap_or(src.len()); // Rest of source is same value when None

            // Fill block values in bulk.
            let value = palette[index.to_usize().unwrap()];
            dst[0..len].fill(value);

            src = src.cut(len..);
            dst = &mut dst[len..];
        }
    }

    #[inline(never)]
    fn set_blocks_core<T: 'static + PrimInt, const BPV: u8>(
        &mut self,
        offset: BlockCoord,
        size: BlockSize,
        src_offset: BlockCoord,
        src_bounds: BlockSize,
        src: &[BlockId],
    ) {
        let src_width = src_bounds.width;
        let src_depth = src_bounds.depth;

        let dst_width = size.width;
        let dst_height = size.height;
        let dst_depth = size.depth;

        let stride = if dst_depth == src_depth && dst_depth == self.depth().get() {
            dst_width * dst_depth
        } else {
            dst_width
        };

        // TODO: increase buffer size preferrably to layer-wide instead of row-wide, to increase SIMD utilization
        let mut index_buffer: ConstVec<T, BPV> = Default::default();
        index_buffer.extend_with(stride, 0);

        if dst_depth == src_depth && dst_depth == self.width().get() {
            for y in 0..dst_height {
                let src_idx = get_index_base(src_depth, src_width, src_offset.y + y, src_offset.z)
                    + src_offset.x;
                let dst_offset = self.get_offset(offset.x, offset.y + y, offset.z) + src_offset.x;
                self.set_contiguous_blocks(
                    src.cut(src_idx..(src_idx + stride)),
                    index_buffer.as_span_mut().cut(dst_offset..),
                );
            }
        } else {
            for y in 0..dst_height {
                let src_y = src_offset.y + y;
                for z in 0..dst_depth {
                    let src_idx = get_index_base(src_depth, src_width, src_y, src_offset.z + z)
                        + src_offset.x;
                    let dst_offset = self.get_offset(offset.x, offset.y + y, offset.z + z);
                    self.set_contiguous_blocks(
                        src.cut(src_idx..(src_idx + stride)),
                        index_buffer.as_span_mut().cut(dst_offset..),
                    );
                }
            }
        }
    }

    fn set_contiguous_blocks<T: PrimInt, const BPV: u8>(
        &mut self,
        mut src: &[BlockId],
        mut index_buffer: PackSpan<&mut [T], ConstPackOrder<T, BPV>>,
    ) {
        assert_eq!(index_buffer.len(), src.len());

        // Unpack block indices in bulk.
        self.data.copy_to(&mut index_buffer);

        let mut buf_idx = 0;
        while src.len() > 0 {
            let value = src[0];
            let (pal_index, _) = self.get_or_add_index(value);
            let pal_value = T::from(*pal_index).unwrap();

            // Move ahead while there are duplicates in the source.
            let len = block_index_of_any_except::<4>(src, value);
            let len = len.unwrap_or(src.len()); // Rest of source is same value when None

            // Copy block indices in bulk.
            (&mut index_buffer)
                .cut(buf_idx..(buf_idx + len))
                .fill(pal_value);

            src = &src[len..];
            buf_idx += len;
        }

        // Pack block indices in bulk.
        index_buffer.copy_to(&mut self.data);
    }

    #[inline(never)]
    fn fill_block_core<T: PrimInt>(&mut self, offset: BlockCoord, size: BlockSize, palette_idx: T) {
        let dst_width = size.width;
        let dst_height = size.height;
        let dst_depth = size.depth;

        if dst_depth == self.depth().get() {
            let stride = dst_width * dst_depth;
            for y in 0..dst_height {
                let dst_idx = self.get_offset(offset.x, offset.y + y, offset.z);
                self.fill_contiguous_blocks(dst_idx, stride, palette_idx);
            }
        } else {
            let stride = dst_width;
            for y in 0..dst_height {
                for z in 0..dst_depth {
                    let dst_idx = self.get_offset(offset.x, offset.y + y, offset.z + z);
                    self.fill_contiguous_blocks(dst_idx, stride, palette_idx);
                }
            }
        }
    }

    fn fill_contiguous_blocks<T: PrimInt>(&mut self, dst_idx: usize, len: usize, palette_idx: T) {
        //nint changeCount = _storage.AsBitSpan(dstIdx, count).Fill(value, changeTracking);
        self.data
            .as_span_mut()
            .cut(dst_idx..(dst_idx + len))
            .fill(palette_idx)
    }
}

impl BlockStorage for ChunkPalette {
    fn width(&self) -> NonZeroUsize {
        Chunk::WIDTH
    }

    fn height(&self) -> NonZeroUsize {
        Chunk::HEIGHT
    }

    fn depth(&self) -> NonZeroUsize {
        Chunk::DEPTH
    }

    fn get_at(&self, offset: usize) -> Option<&BlockId> {
        let index = self.data.get::<PalIdx>(offset)?;
        return self
            .indices
            .value(index)
            .unwrap_or_else(|| panic!("array contains unknown index {}.", index))
            .into();
    }

    fn get_slice(
        &self,
        offset: BlockCoord,
        size: BlockSize,
        dst_offset: BlockCoord,
        dst_bounds: BlockSize,
        dst: &mut [BlockId],
    ) {
        match self.data.order().bits_per_value().get() {
            ..=08 => self.get_blocks_core::<u8, 8, 16>(offset, size, dst_offset, dst_bounds, dst),
            ..=16 => self.get_blocks_core::<u16, 16, 8>(offset, size, dst_offset, dst_bounds, dst),
            ..=32 => self.get_blocks_core::<u32, 32, 4>(offset, size, dst_offset, dst_bounds, dst),
            bpv => panic_unsupported_bpv(bpv),
        }
    }

    fn set_at(&mut self, offset: usize, value: BlockId) -> Option<bool> {
        let index = *self.get_or_add_index(value).0;

        let prev_index = self.data.set(offset, index)?;
        // TODO: also return prev value?
        return Some(prev_index != index);
    }

    fn set_slice(
        &mut self,
        offset: BlockCoord,
        size: BlockSize,
        src_offset: BlockCoord,
        src_bounds: BlockSize,
        src: &[BlockId],
    ) {
        let Some(first_value) = src.first() else {
            return;
        };

        // TODO: is this worthwhile?
        // Use large N since we are checking a contiguous slice and exiting early.
        let Some(run_length) = block_index_of_any_except::<16>(src, *first_value) else {
            return self.fill(offset, size, *first_value);
        };

        let added_count_estimate = src.len() - run_length;
        let bits_needed_estimate =
            get_storage_bits_for_palette(self.indices.len() + added_count_estimate);

        match bits_needed_estimate.get() {
            ..=08 => self.set_blocks_core::<u8, 8>(offset, size, src_offset, src_bounds, src),
            ..=16 => self.set_blocks_core::<u16, 16>(offset, size, src_offset, src_bounds, src),
            ..=32 => self.set_blocks_core::<u32, 32>(offset, size, src_offset, src_bounds, src),
            bpv => panic_unsupported_bpv(bpv),
        }
    }

    fn fill(&mut self, offset: BlockCoord, size: BlockSize, value: BlockId) {
        let palette_idx = *self.get_or_add_index(value).0;
        match self.data.order().bits_per_value().get() {
            ..=08 => self.fill_block_core::<u8>(offset, size, palette_idx as u8),
            ..=16 => self.fill_block_core::<u16>(offset, size, palette_idx as u16),
            ..=32 => self.fill_block_core::<u32>(offset, size, palette_idx as u32),
            bpv => panic_unsupported_bpv(bpv),
        }
    }
}

impl ChunkPalette {
    fn get_or_add_index(&mut self, value: BlockId) -> (&PalIdx, bool) {
        let bits_needed = get_storage_bits_for_palette(self.indices.len() + 1);
        let next_index = self.indices.get_next_index();
        match self.indices.map.entry(value) {
            Entry::Occupied(occupied) => (occupied.into_mut(), false),
            Entry::Vacant(vacant) => {
                if self.data.order().bits_per_value() != bits_needed {
                    std::hint::cold_path();
                    self.data = resize_storage(&self.data, bits_needed);
                }
                (vacant.insert(next_index), true)
            }
        }
    }
}

fn resize_storage<T: PrimInt>(data: &PackVec<T>, bits_per_value: PartSize) -> PackVec<T> {
    let bpv = VarPackOrder::new::<T>(bits_per_value);
    let mut new_storage = PackVec::<T>::with_capacity(data.len(), bpv);

    let src_span = data.as_span();
    let dst_span = &mut new_storage.as_span_mut();
    match bpv.bits_per_value().get() {
        ..=08 => src_span.cast_copy_to::<u8, _>(dst_span),
        ..=16 => src_span.cast_copy_to::<u16, _>(dst_span),
        ..=32 => src_span.cast_copy_to::<u32, _>(dst_span),
        bpv => panic_unsupported_bpv(bpv),
    };
    new_storage
}

#[inline(never)]
#[cold]
fn panic_unsupported_bpv(bits_per_value: u8) -> ! {
    panic!("unsupported value {} for bits_per_value.", bits_per_value)
}
