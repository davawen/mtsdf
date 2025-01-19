use std::marker::PhantomData;

use sdl3_sys::gpu::*;

pub type PrimitiveType = SDL_GPUPrimitiveType;
pub type RasterizerState = SDL_GPURasterizerState;
pub type ColorTargetDescription = SDL_GPUColorTargetDescription;
pub type TextureFormat = SDL_GPUTextureFormat;
pub type MultisampleState = SDL_GPUMultisampleState;
pub type DepthStencilState = SDL_GPUDepthStencilState;
pub type VertexBufferDescription = SDL_GPUVertexBufferDescription;
pub type VertexAttribute = SDL_GPUVertexAttribute;
pub type FillMode = SDL_GPUFillMode;

pub type LoadOp = SDL_GPULoadOp;
pub type StoreOp = SDL_GPUStoreOp;
pub type CompareOp = SDL_GPUCompareOp;

use super::Color;

use super::texture::*;

#[derive(Clone)]
pub enum TargetStoreOp<'a> {
    Resolve {
        texture: TextureRef<'a>,
        mip_level: u32,
        layer: u32,
        cycle: bool,
        store: bool
    },
    Store,
    DontCare,
}

#[repr(transparent)]
pub struct ColorTargetInfo<'a> {
    pub target: SDL_GPUColorTargetInfo,
    _lifetime: PhantomData<&'a ()>
}

impl<'a> ColorTargetInfo<'a> {
    pub fn new_to_texture_clear(texture: TextureRef<'a>, clear_color: Color) -> Self {
        Self::new_to_texture(texture, 0, clear_color, LoadOp::CLEAR, TargetStoreOp::Store)
    }

    pub fn new_to_texture(texture: TextureRef<'a>, mip_level: u32, clear_color: Color, load_op: LoadOp, store_op: TargetStoreOp<'a>) -> Self {
        Self::new(texture, mip_level, 0, clear_color, load_op, store_op, false)
    }

    pub unsafe fn from_raw(target: SDL_GPUColorTargetInfo) -> Self {
        Self { target, _lifetime: PhantomData }
    }

    /// - `texture`: The target texture
    /// - `mip_level`: Mip map level used as a target (use 0 for no mip maps)
    /// - `layer_index`: Index used to slice 3d textures or 2d texture arrays
    /// - `clear_color`: Color used if `LoadOp` is `CLEAR`
    /// - `load_op`: Texture load operation
    /// - `store_op`: Texture store operation
    /// - `cycle`: Whether to cycle the texture if it is already bound
    pub fn new(texture: TextureRef<'a>, mip_level: u32, layer_index: u32, clear_color: Color, load_op: LoadOp, store_op: TargetStoreOp<'a>, cycle: bool) -> Self {
        let (resolve_texture, resolve_mip_level, resolve_layer, cycle_resolve_texture, store_op) = match store_op {
            TargetStoreOp::Resolve { texture, mip_level, layer, cycle, store } =>  {
                (texture.ptr, mip_level, layer, cycle, if store { StoreOp::RESOLVE_AND_STORE } else { StoreOp::RESOLVE })
            }
            TargetStoreOp::DontCare => (std::ptr::null_mut(), 0, 0, false, StoreOp::DONT_CARE),
            TargetStoreOp::Store => (std::ptr::null_mut(), 0, 0, false, StoreOp::STORE)
        };

        unsafe { Self::from_raw(SDL_GPUColorTargetInfo {
            texture: texture.ptr,
            mip_level,
            layer_or_depth_plane: layer_index,
            clear_color,
            load_op,
            store_op,
            cycle,
            resolve_texture,
            resolve_layer,
            resolve_mip_level,
            cycle_resolve_texture,
            padding1: 0,
            padding2: 0,
        }) }
    }
}
