

/// A placed rectangle within an atlas texture, in pixel coordinates with the origin at 
/// the top-left. width and height are unpadded dimensions of the rectangle
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Online shelf-packing allocator for a single atlas page.
/// 
/// Divides the atlas into horizontal "shelves" as needed.
/// Each shelf has its height fixed to that of the tallest thing placed on it so far.
/// Items are placed left->right on the shelf. When an item doesn't fit in the remaining
/// width of the current shelf, a new one is made below it and items are placed there.
/// When a shelf wouldn't fit within the remaining atlas height, the atlas is full and 
/// allocate returns None.
pub struct ShelfPacker {
    atlas_width: u32,
    atlas_height: u32,
    padding: u32,
    
    cursor_x: u32,
    shelf_y: u32,
    shelf_height: u32,
}

impl ShelfPacker {
    pub fn new(atlas_width: u32, atlas_height: u32, padding: u32) -> Self {
        Self {
            atlas_width,
            atlas_height,
            padding,
            cursor_x: 0,
            shelf_y: 0,
            shelf_height: 0,
        }
    }
    
    pub fn atlas_width(&self) -> u32 {
        self.atlas_width
    }
    
    pub fn atlas_height(&self) -> u32 {
        self.atlas_height
    }
    
    /// Attempts to place a width x height rect in the atlas. Returns its position if
    /// successful, None if there is no room left in the atlas.
    pub fn allocate(&mut self, width: u32, height: u32) -> Option<Rect> {
        let padded_w = width + self.padding;
        let padded_h = height + self.padding;
        
        if padded_w > self.atlas_width {
            return None;
        }
        
        if self.cursor_x + padded_w > self.atlas_width {
            self.shelf_y += self.shelf_height;
            self.cursor_x = 0;
            self.shelf_height = 0;
        }
        
        if self.shelf_y + padded_h > self.atlas_height {
            return None;
        }
        
        let rect = Rect {
            x: self.cursor_x + self.padding,
            y: self.shelf_y + self.padding,
            width,
            height,
        };
        
        self.cursor_x += padded_w;
        self.shelf_height = self.shelf_height.max(padded_h);
        
        Some(rect)
    }
}