use std::marker::PhantomData;

use bitflags::bitflags;
use sdl3_sys::gpu::*;

use crate::error::{ErrorKind, Result};

use super::{BufferUsage, CopyPass, Device, StorageTextureReadWriteBinding, TextureFormat, UploadTransferBuffer};

pub struct Texture<'d> {
    pub ptr: *mut SDL_GPUTexture,
    format: TextureFormat,
    width: u32,
    height: u32,
    depth: u32,
    device: &'d Device
}

bitflags! {
    /// Specifies how a texture is intended to be used by the client.
    ///
    /// A texture must have at least one usage flag. Note that some usage flag
    /// combinations are invalid.
    ///
    /// With regards to compute storage usage, READ | WRITE means that you can have
    /// shader A that only writes into the texture and shader B that only reads
    /// from the texture and bind the same texture to either shader respectively.
    /// SIMULTANEOUS means that you can do reads and writes within the same shader
    /// or compute pass. It also implies that atomic ops can be used, since those
    /// are read-modify-write operations. If you use SIMULTANEOUS, you are
    /// responsible for avoiding data races, as there is no data synchronization
    /// within a compute pass. Note that SIMULTANEOUS usage is only supported by a
    /// limited number of texture formats.
    pub struct TextureUsage: u32 {
        /// Texture supports sampling.
        const Sampler = SDL_GPU_TEXTUREUSAGE_SAMPLER;
        /// Texture is a color render target.
        const ColorTarget = SDL_GPU_TEXTUREUSAGE_COLOR_TARGET;
        /// Texture is a depth stencil target.
        const DepthStencilTarget = SDL_GPU_TEXTUREUSAGE_DEPTH_STENCIL_TARGET;
        /// Texture supports storage reads in graphics stages.
        const GraphicsStorageRead = SDL_GPU_TEXTUREUSAGE_GRAPHICS_STORAGE_READ;
        /// Texture supports storage reads in the compute stage.
        const ComputeStorageRead = SDL_GPU_TEXTUREUSAGE_COMPUTE_STORAGE_READ;
        /// Texture supports storage writes in the compute stage.
        const ComputeStorageWrite = SDL_GPU_TEXTUREUSAGE_COMPUTE_STORAGE_WRITE;
        /// Texture supports reads and writes in the same compute shader. This is NOT equivalent to READ | WRITE.
        const ComputeStorageSimultanous = SDL_GPU_TEXTUREUSAGE_COMPUTE_STORAGE_SIMULTANEOUS_READ_WRITE;
    }
}

/// Specifies the sample count of a texture.
///
/// Used in multisampling. Note that this value only applies when the texture
/// is used as a render target.
///
/// ### See also
/// - [`Texture::new`]
/// - [`SampleCount::supported`]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SampleCount {
    /// Normal sampling
    #[default]
    ONE = SDL_GPUSampleCount::_1.0 as isize,
    /// MSAA x2
    TWO = SDL_GPUSampleCount::_2.0 as isize,
    /// MSAA x4
    FOUR = SDL_GPUSampleCount::_4.0 as isize,
    /// MSAA x8
    EIGHT = SDL_GPUSampleCount::_8.0 as isize
}

impl SampleCount {
    /// Checks if the given sample count is supported with the given format and
    /// on the given device.
    pub fn supported(self, device: &Device, format: TextureFormat) -> bool {
        unsafe {
            SDL_GPUTextureSupportsSampleCount(device.ptr, format, self.to_ffi())
        }
    }

    pub unsafe fn to_ffi(self) -> SDL_GPUSampleCount {
        SDL_GPUSampleCount(self as i32)
    }
}

/// Specifies the dimensionality of a texture.;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureType {
    /// 2-dimensional image
    Dim2D = SDL_GPUTextureType::_2D.0 as isize,
    /// Array of 2-dimensional images
    Dim2DArray = SDL_GPUTextureType::_2D_ARRAY.0 as isize,
    /// 3-dimensional volume
    Dim3D = SDL_GPUTextureType::_3D.0 as isize,
    /// Cube image
    Cube = SDL_GPUTextureType::CUBE.0 as isize,
    /// Cube array image
    CubeArray = SDL_GPUTextureType::CUBE_ARRAY.0 as isize
}

impl TextureType {
    pub unsafe fn to_ffi(self) -> SDL_GPUTextureType {
        SDL_GPUTextureType(self as i32)
    }
}

impl<'d> Texture<'d> {
    /// Creates a texture object to be used in graphics or compute workflows.
    /// The contents of this texture are undefined until data is written to the
    /// texture.
    /// 
    /// Note that certain combinations of usage flags are invalid. For example, a
    /// texture cannot have both the SAMPLER and GRAPHICS_STORAGE_READ flags.
    /// 
    /// If you request a sample count higher than the hardware supports, the
    /// implementation will automatically fall back to the highest available sample
    /// count.
    pub fn new(device: &'d Device, format: TextureFormat, ty: TextureType, width: u32, height: u32, depth: u32, usage: TextureUsage, num_mipmaps: u32, msaa: SampleCount) -> Result<Self> {
        unsafe {
            let create_info = SDL_GPUTextureCreateInfo {
                r#type: ty.to_ffi(),
                format, width, height, layer_count_or_depth: depth,
                num_levels: num_mipmaps,
                sample_count: msaa.to_ffi(), usage: usage.bits(),
                props: 0
            };

            let ptr = SDL_CreateGPUTexture(device.ptr, &raw const create_info);

            if ptr.is_null() {
                return Err(ErrorKind::TextureCreation.open())
            }

            Ok(Texture { ptr, format, device, width, height, depth })
        }
    }

    pub fn width(&self) -> u32 { self.width }
    pub fn height(&self) -> u32 { self.height }
    pub fn depth(&self) -> u32 { self.depth }

    /// Fills part of a texture from transfer data.
    /// The data in the transfer buffer should be aligned to the texel size of the texture format.
    ///
    /// The transfer buffer needs to be long enough, but it can be longer than needed.
    /// Data will begin in the transfer buffer at the `transfer_offset` parameter.
    ///
    /// # Panics
    /// Panics if the transfer offset is out of bounds.
    /// Panics if the transfer buffer is too small to fill the texture region.
    pub fn fill_from_transfer_buffer<T: Copy>(
        &self, copy_pass: &CopyPass, 
        transfer_buffer: &UploadTransferBuffer<T>,
        transfer_offset: usize,
        x: u32, y: u32, z: u32, w: u32, h: u32, d: u32,
        mip_level: u32, layer: u32,
        cycle: bool
    ) {
        if transfer_offset > transfer_buffer.len() {
            panic!("out of bounds access to transfer buffer while writing to GPU buffer.\nlen is {}, tried to write from offset {}", 
                transfer_buffer.len(), transfer_offset
            );
        }

        let size = unsafe { SDL_CalculateGPUTextureFormatSize(self.format, self.width, self.height, self.depth) };
        let transfer_size = (transfer_buffer.len() - transfer_offset)*std::mem::size_of::<T>();
        if transfer_size < size as usize {
            panic!("transfer buffer too small to fill GPU buffer.\nBuffer size is {transfer_size}, texture needs at least {size}");
        }

        let info = SDL_GPUTextureTransferInfo {
            transfer_buffer: transfer_buffer.ptr,
            offset: transfer_offset as u32,
            pixels_per_row: w, rows_per_layer: h
        };

        let region = SDL_GPUTextureRegion {
            texture: self.ptr,
            layer, mip_level,
            x, y, z, w, h, d
        };

        unsafe {
            SDL_UploadToGPUTexture(copy_pass.ptr, &raw const info, &raw const region, cycle);
        }
    }

    /// Fills a GPU texure region from a slice.
    /// This creates a transfer buffer, fills it with the slice,
    /// copies it to the texture, and destroys it.
    /// 
    /// # Panics
    /// Panics if the slice is too small to fill the texture region.
    pub fn fill_from_slice<T: Copy>(
        &self, copy_pass: &CopyPass, data: &[T],
        x: u32, y: u32, z: u32, w: u32, h: u32, d: u32,
        mip_level: u32, layer: u32, cycle: bool
    ) -> Result<()> {
        let mut transfer_buffer = UploadTransferBuffer::new(self.device, data.len())?;
        transfer_buffer.fill_from_slice(self.device, data, 0)?;
        self.fill_from_transfer_buffer(copy_pass, &transfer_buffer, 0, x, y, z, w, h, d, mip_level, layer, cycle);
        Ok(())
    }

    /// Gets a borrowed reference to this texture.
    /// For most operations, you only need a [`TextureRef`].
    fn as_ref<'a>(&'a self) -> TextureRef<'a> {
        TextureRef {
            ptr: self.ptr,
            width: self.width,
            height: self.height,
            depth: self.height,
            _lifetime: PhantomData
        }
    }
}

impl Drop for Texture<'_> {
    fn drop(&mut self) {
        unsafe {
            SDL_ReleaseGPUTexture(self.device.ptr, self.ptr);
        }
    }
}

/// A reference to a texture that should not be freed
#[derive(Clone)]
pub struct TextureRef<'a> {
    pub ptr: *mut SDL_GPUTexture,
    width: u32,
    height: u32,
    depth: u32,
    _lifetime: PhantomData<&'a ()>
}

impl<'a> TextureRef<'a> {
    pub unsafe fn from_raw_parts(ptr: *mut SDL_GPUTexture, width: u32, height: u32, depth: u32) -> Self {
        Self { ptr, width, height, depth, _lifetime: PhantomData }
    }

    pub fn width(&self) -> u32 { self.width }
    pub fn height(&self) -> u32 { self.height }
    pub fn depth(&self) -> u32 { self.depth }
}

impl TextureRef<'_> {
    /// Create a read-write binding from this texture.
    /// - `layer_index`: The 3d texture or 2d texture array index
    /// - `mipmap_level`: Which texture mipmap level to use (or 0 for no mipmaps)
    /// - `cycle`: Wether to cycle the texture if it is already bound
    pub fn read_write_binding(&self, layer_index: u32, mipmap_level: u32, cycle: bool) -> StorageTextureReadWriteBinding {
        StorageTextureReadWriteBinding { 
            inner: SDL_GPUStorageTextureReadWriteBinding {
                texture: self.ptr, mip_level: mipmap_level, layer: layer_index, cycle,
                padding1: 0, padding2: 0, padding3: 0
            },
            _lifetime: PhantomData
        }
    } 
}

