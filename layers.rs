// Copyright 2013 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use texturegl::Texture;
use quadtree::{Quadtree};
use platform::surface::{NativeSurface, NativeSurfaceMethods};

use geom::matrix::{Matrix4, identity};
use geom::size::Size2D;
use geom::rect::Rect;
use geom::point::{TypedPoint2D};

use std::cell::RefCell;
use std::fmt::{Formatter, Result, Show};
use std::rc::Rc;

pub enum Format {
    ARGB32Format,
    RGB24Format
}

#[deriving(Clone)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}


#[deriving(Clone)]
pub enum Layer {
    ContainerLayerKind(Rc<ContainerLayer>),
    TextureLayerKind(Rc<TextureLayer>),
    CompositorLayerKind(Rc<CompositorLayer>),
}

impl Layer {
    pub fn with_common<T>(&self, f: |&mut CommonLayer| -> T) -> T {
        match *self {
            ContainerLayerKind(ref container_layer) => {
                f(&mut *container_layer.common.borrow_mut())
            },
            TextureLayerKind(ref texture_layer) => {
                f(&mut *texture_layer.common.borrow_mut())
            },
            CompositorLayerKind(ref compositor_layer) => {
                f(&mut *compositor_layer.container_layer.common.borrow_mut())
            },

        }
    }
}

pub struct CommonLayer {
    pub parent: Option<Layer>,
    pub prev_sibling: Option<Layer>,
    pub next_sibling: Option<Layer>,

    pub transform: Matrix4<f32>,
}

impl CommonLayer {
    // FIXME: Workaround for cross-crate bug regarding mutability of class fields
    pub fn set_transform(&mut self, new_transform: Matrix4<f32>) {
        self.transform = new_transform;
    }
}

pub fn CommonLayer() -> CommonLayer {
    CommonLayer {
        parent: None,
        prev_sibling: None,
        next_sibling: None,
        transform: identity(),
    }
}


pub struct ContainerLayer {
    pub common: RefCell<CommonLayer>,
    pub first_child: RefCell<Option<Layer>>,
    pub last_child: RefCell<Option<Layer>>,
    pub scissor: RefCell<Option<Rect<f32>>>,
}


pub fn ContainerLayer() -> ContainerLayer {
    ContainerLayer {
        common: RefCell::new(CommonLayer()),
        first_child: RefCell::new(None),
        last_child: RefCell::new(None),
        scissor: RefCell::new(None),
    }
}

pub struct ChildIterator {
    current: Option<Layer>,
}

impl Iterator<Layer> for ChildIterator {
    fn next(&mut self) -> Option<Layer> {
        let (new_current, result) =
            match self.current {
                None => (None, None),
                Some(ref child) => {
                    (child.with_common(|x| x.next_sibling.clone()),
                     Some(child.clone()))
                }
            };
        self.current = new_current;
        result
    }
}

impl ContainerLayer {
    pub fn children(&self) -> ChildIterator {
        ChildIterator {
            current: self.first_child.borrow().clone(),
        }
    }

    /// Adds a child to the beginning of the list.
    /// Only works when the child is disconnected from the layer tree.
    pub fn add_child_start(pseudo_self: Rc<ContainerLayer>, new_child: Layer) {
        new_child.with_common(|new_child_common| {
            assert!(new_child_common.parent.is_none());
            assert!(new_child_common.prev_sibling.is_none());
            assert!(new_child_common.next_sibling.is_none());

            new_child_common.parent = Some(ContainerLayerKind(pseudo_self.clone()));

            match *pseudo_self.first_child.borrow() {
                None => {}
                Some(ref first_child) => {
                    first_child.with_common(|first_child_common| {
                        assert!(first_child_common.prev_sibling.is_none());
                        first_child_common.prev_sibling = Some(new_child.clone());
                        new_child_common.next_sibling = Some(first_child.clone());
                    });
                }
            }

            *pseudo_self.first_child.borrow_mut() = Some(new_child.clone());

            let should_set = pseudo_self.last_child.borrow().is_none();
            if should_set {
                *pseudo_self.last_child.borrow_mut() = Some(new_child.clone());
            }
        });
    }

    /// Adds a child to the end of the list.
    /// Only works when the child is disconnected from the layer tree.
    pub fn add_child_end(pseudo_self: Rc<ContainerLayer>, new_child: Layer) {
        new_child.with_common(|new_child_common| {
            assert!(new_child_common.parent.is_none());
            assert!(new_child_common.prev_sibling.is_none());
            assert!(new_child_common.next_sibling.is_none());

            new_child_common.parent = Some(ContainerLayerKind(pseudo_self.clone()));


            match *pseudo_self.last_child.borrow() {
                None => {}
                Some(ref last_child) => {
                    last_child.with_common(|last_child_common| {
                        assert!(last_child_common.next_sibling.is_none());
                        last_child_common.next_sibling = Some(new_child.clone());
                        new_child_common.prev_sibling = Some(last_child.clone());
                    });
                }
            }

            *pseudo_self.last_child.borrow_mut() = Some(new_child.clone());

            let mut child = pseudo_self.first_child.borrow_mut();
            match *child {
                Some(_) => {},
                None => *child = Some(new_child.clone()),
            }
        });
    }
    
    pub fn remove_child(pseudo_self: Rc<ContainerLayer>, child: Layer) {
        child.with_common(|child_common| {
            assert!(child_common.parent.is_some());
            match child_common.parent {
                Some(ContainerLayerKind(ref container)) => {
                    assert!(container.deref() as *ContainerLayer ==
                            pseudo_self.deref() as *ContainerLayer);
                },
                _ => fail!("Invalid parent of child in layer tree"),
            }

            match child_common.next_sibling {
                None => { // this is the last child
                    *pseudo_self.last_child.borrow_mut() = child_common.prev_sibling.clone();
                },
                Some(ref sibling) => {
                    sibling.with_common(|sibling_common| {
                        sibling_common.prev_sibling = child_common.prev_sibling.clone();
                    });
                }
            }
            match child_common.prev_sibling {
                None => { // this is the first child
                    *pseudo_self.first_child.borrow_mut() = child_common.next_sibling.clone();
                },
                Some(ref sibling) => {
                    sibling.with_common(|sibling_common| {
                        sibling_common.next_sibling = child_common.next_sibling.clone();
                    });
                }
            }           
        });
    }

    pub fn remove_all_children(&self) {
        *self.first_child.borrow_mut() = None;
        *self.last_child.borrow_mut() = None;
    }
}

/// Whether a texture should be flipped.
#[deriving(PartialEq)]
pub enum Flip {
    /// The texture should not be flipped.
    NoFlip,
    /// The texture should be flipped vertically.
    VerticalFlip,
}

pub struct TextureLayer {
    pub common: RefCell<CommonLayer>,
    /// A handle to the GPU texture.
    pub texture: Texture,
    /// The size of the texture in pixels.
    size: Size2D<uint>,
    /// Whether this texture is flipped vertically.
    pub flip: Flip,
}

impl TextureLayer {
    pub fn new(texture: Texture, size: Size2D<uint>, flip: Flip) -> TextureLayer {
        TextureLayer {
            common: RefCell::new(CommonLayer()),
            texture: texture,
            size: size,
            flip: flip,
        }
    }
}


#[deriving(Clone, Eq)]
pub struct LayerId(pub uint, pub uint);

impl Show for LayerId {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let LayerId(a, b) = *self;
        write!(f, "Layer({}, {})", a, b)
    }
}

impl LayerId {
    /// FIXME(#2011, pcwalton): This is unfortunate. Maybe remove this in the future.
    pub fn null() -> LayerId {
        LayerId(0, 0)
    }
}

/// The scrolling policy of a layer.
#[deriving(Eq)]
pub enum ScrollPolicy {
    /// These layers scroll when the parent receives a scrolling message.
    Scrollable,
    /// These layers do not scroll when the parent receives a scrolling message.
    FixedPosition,
}

#[deriving(Eq, Clone)]
pub enum WantsScrollEventsFlag {
    WantsScrollEvents,
    DoesntWantScrollEvents,
}

/// One CSS "px" in the root coordinate system for the content document.
///
/// PagePx is equal to ViewportPx multiplied by a "viewport zoom" factor controlled by the user.
/// This is the mobile-style "pinch zoom" that enlarges content without reflowing it.  When the
/// viewport zoom is not equal to 1.0, then the layout viewport is no longer the same physical size
/// as the viewable area.
pub enum PagePx {}


pub struct LayerBuffer {
    /// The native surface which can be shared between threads or processes. On Mac this is an
    /// `IOSurface`; on Linux this is an X Pixmap; on Android this is an `EGLImageKHR`.
    pub native_surface: NativeSurface,

    /// The rect in the containing RenderLayer that this represents.
    pub rect: Rect<f32>,

    /// The rect in pixels that will be drawn to the screen.
    pub screen_pos: Rect<uint>,

    /// The scale at which this tile is rendered
    pub resolution: f32,

    /// NB: stride is in pixels, like OpenGL GL_UNPACK_ROW_LENGTH.
    pub stride: uint,

    /// Used by the RenderTask to route buffers to the correct graphics context for recycling
    pub render_idx: uint
}

/// A set of layer buffers. This is an atomic unit used to switch between the front and back
/// buffers.
pub struct LayerBufferSet {
    pub buffers: Vec<Box<LayerBuffer>>
}

impl LayerBufferSet {
    /// Notes all buffer surfaces will leak if not destroyed via a call to `destroy`.
    pub fn mark_will_leak(&mut self) {
        for buffer in self.buffers.mut_iter() {
            buffer.native_surface.mark_will_leak()
        }
    }
}

/// The CompositorLayer represents an element on a page that has a unique scroll
/// or animation behavior. This can include absolute positioned elements, iframes, etc.
/// Each layer can also have child layers.
pub struct CompositorLayer {
    pub container_layer: ContainerLayer,
    pub pipeline_id: uint, // maybe can remove?
    pub id: LayerId,

    /// This layer's quadtree. This is where all buffers are stored for this layer.
    pub quadtree: MaybeQuadtree,

    /// The size of the underlying page in page coordinates. This is an option
    /// because we may not know the size of the page until layout is finished completely.
    /// if we have no size yet, the layer is hidden until a size message is recieved.
    pub page_size: Option<Size2D<f32>>,

    /// The offset of the page due to scrolling. (0,0) is when the window sees the
    /// top left corner of the page.
    pub scroll_offset: TypedPoint2D<PagePx, f32>,

    /// This layer's quadtree. This is where all buffers are stored for this layer.
    //pub quadtree: MaybeQuadtree,

    /// When set to true, this layer is ignored by its parents. This is useful for
    /// soft deletion or when waiting on a page size.
    pub hidden: bool,

    /// Whether an ancestor layer that receives scroll events moves this layer.
    pub scroll_policy: ScrollPolicy,

    /// True if CPU rendering is enabled, false if we're using GPU rendering.
    pub cpu_painting: bool,

    /// A monotonically increasing counter that keeps track of the current epoch.
    /// add_buffer() calls that don't match the current epoch will be ignored.
    pub epoch: uint, //Epoch, //maybe can remove?

    /// The behavior of this layer when a scroll message is received.
    pub wants_scroll_events: WantsScrollEventsFlag,

    /// The color to use for the unrendered-content void
    pub unrendered_color: Color,
}

/// Helper enum for storing quadtrees. Either contains a quadtree, or contains
/// information from which a quadtree can be built.
pub enum MaybeQuadtree {
    Tree(Quadtree<Box<LayerBuffer>>),
    NoTree(uint, Option<uint>),
}

impl MaybeQuadtree {
    pub fn tile_size(&self) -> uint {
        match *self {
            Tree(ref quadtree) => quadtree.max_tile_size,
            NoTree(tile_size, _) => tile_size,
        }
    }
}
