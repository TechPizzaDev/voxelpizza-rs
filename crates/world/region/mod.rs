use bevy::prelude::Component;

#[derive(Component, Debug, Default)]
pub struct ChunkRegionLocation {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

#[derive(Component)]
#[require(ChunkRegionLocation)]
pub struct ChunkRegion {
    
}
