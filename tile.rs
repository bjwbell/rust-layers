use geom::size::Size2D;
use layers::LayerBuffer;
use platform::surface::{NativePaintingGraphicsContext};
use platform::surface::{NativeSurfaceMethods};

/// The interface used by the quadtree and buffer map to get info about layer buffers.
pub trait Tile {
    /// Returns the amount of memory used by the tile
    fn get_mem(&self) -> uint;
    /// Returns true if the tile is displayable at the given scale
    fn is_valid(&self, f32) -> bool;
    /// Returns the Size2D of the tile
    fn get_size_2d(&self) -> Size2D<uint>;

    /// Marks the layer buffer as not leaking. See comments on
    /// `NativeSurfaceMethods::mark_wont_leak` for how this is used.
    fn mark_wont_leak(&mut self);

    /// Destroys the layer buffer. Painting task only.
    fn destroy(self, graphics_context: &NativePaintingGraphicsContext);
}

impl Tile for Box<LayerBuffer> {
    fn get_mem(&self) -> uint {
        // This works for now, but in the future we may want a better heuristic
        self.screen_pos.size.width * self.screen_pos.size.height
    }
    fn is_valid(&self, scale: f32) -> bool {
        (self.resolution - scale).abs() < 1.0e-6
    }
    fn get_size_2d(&self) -> Size2D<uint> {
        self.screen_pos.size
    }
    fn mark_wont_leak(&mut self) {
        self.native_surface.mark_wont_leak()
    }
    fn destroy(self, graphics_context: &NativePaintingGraphicsContext) {
        let mut this = self;
        this.native_surface.destroy(graphics_context)
    }
}

