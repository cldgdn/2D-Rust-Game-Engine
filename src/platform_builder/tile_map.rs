use std::rc::Rc;
use std::sync::OnceLock;
use glam::{Mat4, Vec3, Vec4};
use crate::engine::component::transform::Transform;
use crate::engine::{Drawable, Entity, EntityID};
use crate::engine::game::Game;
use crate::engine::rendering::shader::{Shader, ShaderID};
use crate::engine::visual::texture::Texture2D;
use crate::engine::world::World;

const TEXTURE_ID_MASK: u8 = 0b0011_1111;
const SOLID_BIT: u8 = 0b0100_0000;
const MESHED_BIT: u8 = 0b1000_0000;

static LOOKUP: OnceLock<[Option<u8>; 256]> = OnceLock::new();

pub type Tile = u8;

#[derive(Copy, Clone)]
struct MaskPattern {
    mask: u8,
    pattern: u8,
}

pub struct TileMap {
    id: EntityID,
    tiles: Vec<Vec<Tile>>,
    //atlas: Rc<Texture2D>
}

impl TileMap {
    pub fn new(game: &mut Game, solid_map: &[Vec<bool>]) -> Self {
        let id = game.world_mut().get_next_entity_id();

        let height = solid_map.len();
        let width = solid_map[0].len();

        let mut tilemap = TileMap {
            id,
            tiles: vec![vec![0; width]; height],
        };

        tilemap.remap_tiles(solid_map);
        tilemap.greedy_mesh_tiles();

        tilemap
    }

    fn remap_tiles(&mut self, solid_map: &[Vec<bool>]) {
        let height = solid_map.len();
        let width = solid_map[0].len();

        for i in 0..height {
            for j in 0..width {
                let mut neighbours = self.map_neighbours(solid_map, i, j);

                if !solid_map[i][j] {
                    neighbours = !neighbours;
                }

                let table = lookup();

                if let Some(id) = table[neighbours as usize] {
                    self.tiles[i][j] = id + if solid_map[i][j] {SOLID_BIT} else {0};
                } else {
                    self.tiles[i][j] = SOLID_BIT;
                    eprintln!("Failed to match tile at ({i}, {j}) with neighbours {neighbours:b}");
                }
            }
        }
    }

    fn greedy_mesh_tiles(&mut self) {
        
    }
    
    fn greedy_textures(&mut self) {
        
    }
    
    fn greedy_colliders(&mut self) {
        
    }

    fn map_neighbours(&self, solid_map: &[Vec<bool>], x: usize, y: usize) -> u8 {
        let height = solid_map.len() as isize;
        let width = solid_map[0].len() as isize;

        let mut neighbours: u8 = 0;
        let mut curr_bit: u8 = 0b1000_0000;

        for i in (x as isize - 1)..=(x as isize + 1) {
            for j in (y as isize - 1)..=(y as isize + 1) {
                if i == x as isize && j == y as isize {
                    continue;
                }

                if i < 0
                    || i >= height
                    || j < 0
                    || j >= width
                    || solid_map[i as usize][j as usize]
                {
                    neighbours |= curr_bit;
                }

                curr_bit >>= 1;
            }
        }

        neighbours
    }
}

impl Entity for TileMap {
    fn id(&self) -> EntityID {
        self.id
    }
}

fn build_lookup() -> [Option<u8>; 256] {
    let mut lookup = [None; 256];

    for neighbours in 0u8..=255 {
        for (k, map) in MASK_TO_ID.iter().enumerate() {
            if (neighbours & map.mask) == map.pattern {
                lookup[neighbours as usize] = Some(k as u8);
                break;
            }
        }
    }

    lookup
}

fn lookup() -> & 'static [Option<u8>; 256] {
    LOOKUP.get_or_init(build_lookup)
}

const MASK_TO_ID: &[MaskPattern] = &[
    MaskPattern {mask: 0b01011010, pattern: 0b00000000},
    MaskPattern {mask: 0b01011010, pattern: 0b00010000},
    MaskPattern {mask: 0b01011010, pattern: 0b01000000},
    MaskPattern {mask: 0b01011010, pattern: 0b00001000},
    MaskPattern {mask: 0b01011010, pattern: 0b00000010},
    MaskPattern {mask: 0b01011110, pattern: 0b00010110},
    MaskPattern {mask: 0b01011110, pattern: 0b00010010},
    MaskPattern {mask: 0b11011010, pattern: 0b11010000},
    MaskPattern {mask: 0b11011010, pattern: 0b01010000},
    MaskPattern {mask: 0b01111010, pattern: 0b01101000},
    MaskPattern {mask: 0b01111010, pattern: 0b01001000},
    MaskPattern {mask: 0b01011011, pattern: 0b00001011},
    MaskPattern {mask: 0b01011011, pattern: 0b00001010},
    MaskPattern {mask: 0b01011111, pattern: 0b00011111},
    MaskPattern {mask: 0b01011111, pattern: 0b00011011},
    MaskPattern {mask: 0b01011111, pattern: 0b00011110},
    MaskPattern {mask: 0b01011111, pattern: 0b00011010},
    MaskPattern {mask: 0b11011110, pattern: 0b11010110},
    MaskPattern {mask: 0b11011110, pattern: 0b01010110},
    MaskPattern {mask: 0b11011110, pattern: 0b11010010},
    MaskPattern {mask: 0b11011110, pattern: 0b01010010},
    MaskPattern {mask: 0b11111010, pattern: 0b11111000},
    MaskPattern {mask: 0b11111010, pattern: 0b11011000},
    MaskPattern {mask: 0b11111010, pattern: 0b01111000},
    MaskPattern {mask: 0b11111010, pattern: 0b01011000},
    MaskPattern {mask: 0b01111011, pattern: 0b01101011},
    MaskPattern {mask: 0b01111011, pattern: 0b01001011},
    MaskPattern {mask: 0b01111011, pattern: 0b01101010},
    MaskPattern {mask: 0b01111011, pattern: 0b01001010},
    MaskPattern {mask: 0b01011010, pattern: 0b00011000},
    MaskPattern {mask: 0b01011010, pattern: 0b01000010},
    MaskPattern {mask: 0b11111111, pattern: 0b11111111},
    MaskPattern {mask: 0b11111111, pattern: 0b11011111},
    MaskPattern {mask: 0b11111111, pattern: 0b11111110},
    MaskPattern {mask: 0b11111111, pattern: 0b11111011},
    MaskPattern {mask: 0b11111111, pattern: 0b01111111},
    MaskPattern {mask: 0b11111111, pattern: 0b01011111},
    MaskPattern {mask: 0b11111111, pattern: 0b11011110},
    MaskPattern {mask: 0b11111111, pattern: 0b11111010},
    MaskPattern {mask: 0b11111111, pattern: 0b01111011},
    MaskPattern {mask: 0b11111111, pattern: 0b01011110},
    MaskPattern {mask: 0b11111111, pattern: 0b11011010},
    MaskPattern {mask: 0b11111111, pattern: 0b01111010},
    MaskPattern {mask: 0b11111111, pattern: 0b01011011},
    MaskPattern {mask: 0b11111111, pattern: 0b01011010},
    MaskPattern {mask: 0b11111111, pattern: 0b01111110},
    MaskPattern {mask: 0b11111111, pattern: 0b11011011}
];