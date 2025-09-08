pub mod palette;

use std::num::NonZeroUsize;

use bevy::prelude::Component;
use collections::RangeCut;
use num_traits::PrimInt;
use palette::ChunkPalette;

use crate::block::{BlockId, BlockCoord, BlockSize};

#[derive(Component, Debug, Default)]
pub struct ChunkLocation {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

#[derive(Component, Debug)]
#[require(ChunkLocation)]
pub struct Chunk {
    storage: ChunkStorage,
}

impl Chunk {
    pub const WIDTH: NonZeroUsize = NonZeroUsize::new(16).unwrap();
    pub const HEIGHT: NonZeroUsize = NonZeroUsize::new(16).unwrap();
    pub const DEPTH: NonZeroUsize = NonZeroUsize::new(16).unwrap();
}

#[derive(Debug)]
pub enum ChunkStorage {
    Empty,

    /// A single value represents the entire storage.
    Value(BlockId),

    Palette(ChunkPalette),
}

pub trait BlockStorage {
    fn width(&self) -> NonZeroUsize;

    fn height(&self) -> NonZeroUsize;

    fn depth(&self) -> NonZeroUsize;

    fn size(&self) -> BlockSize {
        return BlockSize {
            width: self.width().get(),
            height: self.height().get(),
            depth: self.depth().get(),
        };
    }

    fn get_offset(&self, x: usize, y: usize, z: usize) -> usize {
        return get_index_base(self.depth().get(), self.width().get(), y, z) + x;
    }

    fn get_coord_offset(&self, offset: BlockCoord) -> usize {
        return self.get_offset(offset.x, offset.y, offset.z);
    }

    fn get_at(&self, offset: usize) -> Option<&BlockId>;

    fn get_slice(
        &self,
        offset: BlockCoord,
        size: BlockSize,
        dst_offset: BlockCoord,
        dst_bounds: BlockSize,
        dst: &mut [BlockId],
    );

    fn set_at(&mut self, offset: usize, value: BlockId) -> Option<bool>;

    fn set_slice(
        &mut self,
        offset: BlockCoord,
        size: BlockSize,
        src_offset: BlockCoord,
        src_bounds: BlockSize,
        src: &[BlockId],
    );

    fn fill(&mut self, offset: BlockCoord, size: BlockSize, value: BlockId);
}

impl BlockStorage for Chunk {
    fn width(&self) -> NonZeroUsize {
        Self::WIDTH
    }

    fn height(&self) -> NonZeroUsize {
        Self::HEIGHT
    }

    fn depth(&self) -> NonZeroUsize {
        Self::DEPTH
    }

    fn get_at(&self, offset: usize) -> Option<&BlockId> {
        match &self.storage {
            ChunkStorage::Empty => None,
            ChunkStorage::Value(value) => Some(value),
            ChunkStorage::Palette(palette) => palette.get_at(offset),
        }
    }

    fn get_slice(
        &self,
        offset: BlockCoord,
        size: BlockSize,
        dst_offset: BlockCoord,
        dst_bounds: BlockSize,
        dst: &mut [BlockId],
    ) {
        match &self.storage {
            ChunkStorage::Empty => fill(dst_offset, size, BlockId::default(), dst_bounds, dst),
            ChunkStorage::Value(value) => fill(dst_offset, size, *value, dst_bounds, dst),
            ChunkStorage::Palette(palette) => {
                palette.get_slice(offset, size, dst_offset, dst_bounds, dst)
            }
        }
    }

    fn set_at(&mut self, offset: usize, value: BlockId) -> Option<bool> {
        match &mut self.storage {
            ChunkStorage::Palette(palette) => palette.set_at(offset, value),
            _ => todo!(),
        }
    }

    fn set_slice(
        &mut self,
        offset: BlockCoord,
        size: BlockSize,
        src_offset: BlockCoord,
        src_bounds: BlockSize,
        src: &[BlockId],
    ) {
        match &mut self.storage {
            ChunkStorage::Palette(palette) => {
                palette.set_slice(offset, size, src_offset, src_bounds, src)
            }
            _ => todo!(),
        }
    }

    fn fill(&mut self, offset: BlockCoord, size: BlockSize, value: BlockId) {
        match &mut self.storage {
            ChunkStorage::Palette(palette) => palette.fill(offset, size, value),
            _ => todo!(),
        }
    }
}

#[inline(always)]
pub const fn get_index_base(depth: usize, width: usize, y: usize, z: usize) -> usize {
    return (y * depth + z) * width;
}

pub fn fill<T>(
    offset: BlockCoord,
    size: BlockSize,
    value: T,
    dst_bounds: BlockSize,
    dst: &mut [T],
) where
    T: Copy,
{
    for y in 0..size.height {
        let dst_y = offset.y + y;

        for z in 0..size.depth {
            let dst_idx = get_index_base(dst_bounds.depth, dst_bounds.width, dst_y, offset.z + z);
            let dst_slice = dst.cut(dst_idx + offset.x, size.width);

            dst_slice.fill(value);
        }
    }
}

pub fn cast_copy<S, D>(
    src_offset: BlockCoord,
    src_bounds: BlockSize,
    src: &[S],
    dst_offset: BlockCoord,
    dst_bounds: BlockSize,
    dst: &mut [D],
    copy_size: BlockSize,
) where
    S: PrimInt,
    D: PrimInt,
{
    for y in 0..copy_size.height {
        let src_y = src_offset.y + y;
        let dst_y = dst_offset.y + y;

        for z in 0..copy_size.depth {
            let src_z = src_offset.z + z;
            let src_idx = get_index_base(src_bounds.depth, src_bounds.width, src_y, src_z);
            let src_slice = src.cut(src_idx + src_offset.x, copy_size.width);

            let dst_z = dst_offset.z + z;
            let dst_idx = get_index_base(dst_bounds.depth, dst_bounds.width, dst_y, dst_z);
            let dst_slice = dst.cut(dst_idx + dst_offset.x, copy_size.width);

            cast(src_slice, dst_slice);
        }
    }
}

pub fn cast<S, D>(src: &[S], dst: &mut [D])
where
    S: PrimInt,
    D: PrimInt,
{
    for i in 0..src.len() {
        dst[i] = D::from(src[i]).unwrap();
    }
}

#[cfg(test)]
mod tests {

    /* 
    let size = black_box(64);
    let src = vec![256; size];
    let mut dst8 = vec![0 as u8; size];
    let mut dst16 = vec![0 as u16; size];
    world::chunk::convert(&src, &mut dst8);
    world::chunk::convert(&dst8, &mut dst16);
    println!("{:?}", src);
    println!("{:?}", dst8);
    println!("{:?}", dst16);
    */
}