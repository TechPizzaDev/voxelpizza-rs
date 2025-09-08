use bytemuck::NoUninit;


#[derive(Clone, Copy, Debug, Default, Hash, Eq, PartialEq, NoUninit)]
#[repr(transparent)]
pub struct BlockId(pub u32);

#[derive(Clone, Copy, Default, Hash, Eq, PartialEq)]
pub struct BlockCoord {
    pub x: usize,
    pub y: usize,
    pub z: usize,
}

impl BlockCoord {
    pub fn splat(value: usize) -> Self {
        Self {
            x: value,
            y: value,
            z: value,
        }
    }
}

#[derive(Clone, Copy, Default, Hash, Eq, PartialEq)]
pub struct BlockSize {
    pub width: usize,
    pub height: usize,
    pub depth: usize,
}

impl BlockSize {
    pub fn splat(value: usize) -> Self {
        Self {
            width: value,
            height: value,
            depth: value,
        }
    }

    pub fn volume(self) -> usize {
        self.width * self.height * self.depth
    }
}
