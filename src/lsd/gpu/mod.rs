use std::{ffi::CString, marker::PhantomData};

use sdl3_sys::gpu::*;

use super::{error::{ErrorKind, Result}, log::{log_warn, LogCategory}, Window};

pub use super::Color;

mod primitives;
mod shader;
mod device;
mod buffer;
mod texture;

pub use device::*;
pub use primitives::*;
pub use shader::*;
pub use buffer::*;
pub use texture::*;

#[macro_export]
macro_rules! spirv {
    ($path:literal, $stage:ident) => {
        {
            let slice = inline_spirv::include_spirv!($path, glsl, $stage).as_slice();
            let ptr = slice.as_ptr() as *const u8;
            unsafe { std::slice::from_raw_parts(ptr, slice.len()*4) }
        }
    };
}

pub struct GraphicsPipeline<'a> {
    pub ptr: *mut SDL_GPUGraphicsPipeline,
    shaders: PhantomData<&'a Shader<'a>>
}

impl<'a> GraphicsPipeline<'a> {
    /// Sets up a basic render pipeline with:
    /// - no depth stencil, no depth buffer, no depth bias
    /// - no multisampling
    /// - window swapchain texture as a color target
    /// - back face culling (and front faces are counter clockwise)
    /// - no color blending
    pub fn new_basic(
        device: &Device, window: &Window,
        vertex: &'a Shader, fragment: &'a Shader,
        primitive_type: PrimitiveType, fill_mode: FillMode,
        vertex_buffer_descriptions: &[VertexBufferDescription],
        vertex_attributes: &[VertexAttribute]
    ) -> Result<Self> {
        let target = &[ColorTargetDescription {
            // alpha blending: dstRGB = (srcRGB * srcA) + (dstRGB * (1-srcA)), dstA = srcA + (dstA * (1-srcA))
            format: device.swapchain_texture_format(window),
            blend_state: SDL_GPUColorTargetBlendState {
                enable_blend: true,
                color_blend_op: SDL_GPUBlendOp::ADD,
                alpha_blend_op: SDL_GPUBlendOp::ADD,
                src_color_blendfactor: SDL_GPUBlendFactor::SRC_ALPHA,
                dst_color_blendfactor: SDL_GPUBlendFactor::ONE_MINUS_SRC_ALPHA,
                src_alpha_blendfactor: SDL_GPUBlendFactor::ONE,
                dst_alpha_blendfactor: SDL_GPUBlendFactor::ONE_MINUS_SRC_ALPHA,
                ..(unsafe { std::mem::zeroed() })
            }
        }];

        Self::new(
            device, vertex, fragment, primitive_type,
            RasterizerState {
                cull_mode: SDL_GPUCullMode::BACK,
                front_face: SDL_GPUFrontFace::COUNTER_CLOCKWISE,
                depth_bias_clamp: 0.0,
                depth_bias_constant_factor: 0.0,
                depth_bias_slope_factor: 0.0,
                enable_depth_bias: false,
                enable_depth_clip: false,
                fill_mode,
                padding1: 0, padding2: 0
            },
            target,
            None,
            vertex_buffer_descriptions,
            vertex_attributes,
            None,
            MultisampleState { enable_mask: false, sample_count: SDL_GPUSampleCount::_1, ..(unsafe { std::mem::zeroed() })}
        )
    }

    pub fn new(
        device: &'_ Device,
        vertex: &'a Shader, fragment: &'a Shader,
        primitive_type: PrimitiveType, rasterizer_state: RasterizerState,
        color_targets: &'_ [ColorTargetDescription],
        depth_stencil_format: Option<TextureFormat>,
        vertex_buffer_descriptions: &'_ [VertexBufferDescription],
        vertex_attributes: &'_ [VertexAttribute],
        depth_stencil_state: Option<DepthStencilState>,
        multisample_state: MultisampleState
    ) -> Result<Self> {
        let mut create = SDL_GPUGraphicsPipelineCreateInfo {
            vertex_shader: vertex.ptr,
            fragment_shader: fragment.ptr,
            primitive_type,
            rasterizer_state,
            target_info: SDL_GPUGraphicsPipelineTargetInfo {
                color_target_descriptions: color_targets.as_ptr(),
                num_color_targets: color_targets.len() as u32,
                has_depth_stencil_target: depth_stencil_format.is_some(),
                depth_stencil_format: depth_stencil_format.unwrap_or(TextureFormat::INVALID),
                padding1: 0, padding2: 0, padding3: 0
            },
            vertex_input_state: SDL_GPUVertexInputState {
                vertex_attributes: vertex_attributes.as_ptr(),
                num_vertex_attributes: vertex_attributes.len() as u32,
                vertex_buffer_descriptions: vertex_buffer_descriptions.as_ptr(),
                num_vertex_buffers: vertex_buffer_descriptions.len() as u32
            },
            depth_stencil_state: depth_stencil_state.unwrap_or(unsafe { std::mem::zeroed() }),
            multisample_state,
            props: 0
        };

        unsafe {
            let ptr = SDL_CreateGPUGraphicsPipeline(device.ptr, &mut create as *mut _);
            if ptr.is_null() {
                return Err(ErrorKind::GraphicsPipelineCreation.open());
            }
            Ok(GraphicsPipeline { ptr, shaders: PhantomData })
        }
    }
}

pub struct ComputePipeline<'a> {
    pub ptr: *mut SDL_GPUComputePipeline,
    shader: PhantomData<&'a Shader<'a>>
}

impl<'a> ComputePipeline<'a> {
    /// Creates a compute pipeline from the given shader.
    ///
    /// For SPIR-V code, ressources must specified in the following order:
    /// - In set 0, first bind texture samplers, then read-only textures, then read-only storage buffers.
    /// - In set 1, first bind writeable textures, then writeable storage buffers.
    /// - In set 2, bind uniform buffers.
    /// For exemple:
    /// ```glsl
    /// layout (set = 0, binding = 0) uniform sampler2D my_texture_sampler;
    /// layout (set = 0, binding = 1, rgba32f) readonly uniform image2D my_readonly_texture;
    /// layout (std430, set = 0, binding = 2) buffer InputStorageBuffer { float data[]; } input_buffer;
    /// layout (std430, set = 1, binding = 0) buffer OutputStorageBuffer { float data[]; } output_buffer;
    /// layout (set = 2, binding = 0) uniform { float config_data; };
    /// ```
    pub fn new(
        device: &'a Device,
        shader_code: &[u8], entrypoint: &str, format: ShaderFormat,
        num_samplers: u32, num_uniform_buffers: u32,
        num_readonly_storage_textures: u32, num_readonly_storage_buffers: u32,
        num_readwrite_storage_textures: u32, num_readwrite_storage_buffers: u32,
        threadcount: [u32; 3]) -> Result<Self> {
        let entrypoint = CString::new(entrypoint).unwrap();
        let info = SDL_GPUComputePipelineCreateInfo {
            code: shader_code.as_ptr(),
            code_size: shader_code.len(),
            entrypoint: entrypoint.as_ptr(),
            format: format.bits(),
            num_samplers,
            num_uniform_buffers,
            num_readonly_storage_textures,
            num_readonly_storage_buffers,
            num_readwrite_storage_textures,
            num_readwrite_storage_buffers,
            threadcount_x: threadcount[0],
            threadcount_y: threadcount[1],
            threadcount_z: threadcount[2],
            props: 0
        };
        unsafe {
            let ptr = SDL_CreateGPUComputePipeline(device.ptr, &info as *const _);
            if ptr.is_null() {
                return Err(ErrorKind::ComputePipelineCreation.open());
            }
            Ok(Self { ptr, shader: PhantomData })
        }
    }
}

pub struct CommandBuffer {
    pub ptr: *mut SDL_GPUCommandBuffer
}

pub struct ComputePass {
    pub ptr: *mut SDL_GPUComputePass
}

pub struct RenderPass {
    pub ptr: *mut SDL_GPURenderPass
}

pub struct CopyPass {
    pub ptr: *mut SDL_GPUCopyPass
}

pub struct Fence<'a> {
    pub ptr: *mut SDL_GPUFence,
    device: &'a Device
}

/// A storage texture read write binding.
/// You can create one with the [`TextureRef::read_write_binding`] method.
#[repr(transparent)]
pub struct StorageTextureReadWriteBinding<'a> {
    inner: SDL_GPUStorageTextureReadWriteBinding,
    _lifetime: PhantomData<TextureRef<'a>>
}

/// A storage buffer read write binding.
/// The type contained in the buffer is erased.
/// You can create one with the [`Buffer::read_write_binding`] method.
#[repr(transparent)]
pub struct StorageBufferReadWriteBinding<'a> {
    inner: SDL_GPUStorageBufferReadWriteBinding,
    _lifetime: PhantomData<&'a Buffer<'a, u8>>
}

/// A storage buffer read only binding.
/// The type contained in the buffer is erased.
/// You can create one with the [`Buffer::read_binding`] method.
/// 
/// NOTE: This type is not present in the original SDL3 interface.
#[repr(transparent)]
pub struct StorageBufferReadBinding<'a> {
    inner: *mut SDL_GPUBuffer,
    _lifetime: PhantomData<&'a Buffer<'a, u8>>
}

/// A vertex buffer read only binding.
/// The type contained in the buffer is erased.
/// You can create one with the [`Buffer::vertex_binding`] method.
/// 
/// NOTE: This type is not present in the original SDL3 interface.
#[repr(transparent)]
pub struct VertexBufferBinding<'a> {
    inner: SDL_GPUBufferBinding,
    _lifetime: PhantomData<&'a Buffer<'a, u8>>
}

impl CommandBuffer {
    pub fn acquire_swapchain_texture<'a>(&'a self, window: &Window) -> Result<TextureRef<'a>> {
        unsafe {
            let mut ptr = std::ptr::null_mut();
            let mut width = 0;
            let mut height = 0;
            if !SDL_AcquireGPUSwapchainTexture(self.ptr, window.ptr, &raw mut ptr, &raw mut width, &raw mut height) {
                return Err(ErrorKind::AcquireSwapchainTexture.open())
            }
            Ok(TextureRef::from_raw_parts(ptr, width, height, 1))
        }
    }
    
    /// Begins a compute pass on a command buffer.
    ///
    /// A compute pass is defined by a set of texture subresources and buffers that
    /// may be written to by compute pipelines.
    /// These are passed in the compute pass creation call,
    /// and are bound first and in order.
    /// You can then bind read-only data with [`ComputePass::bind_*`] methods.
    /// These textures and buffers must
    /// have been created with the [`BufferUsage::ComputeStorageWrite`] bit or the
    /// [`TextureUsage::ComputeStorageSimultaneousReadWrite`] bit.
    /// All operations related to compute pipelines
    /// must take place inside of a compute pass. You must not begin another
    /// compute pass, or a render pass or copy pass before ending the compute pass.
    /// 
    /// A VERY IMPORTANT NOTE - Reads and writes in compute passes are NOT
    /// implicitly synchronized. This means you may cause data races by both
    /// reading and writing a resource region in a compute pass, or by writing
    /// multiple times to a resource region. If your compute work depends on
    /// reading the completed output from a previous dispatch, you MUST end the
    /// current compute pass and begin a new one before you can safely access the
    /// data. Otherwise you will receive unexpected results. Reading and writing a
    /// texture in the same compute pass is only supported by specific texture
    /// formats. Make sure you check the format support!
    pub fn begin_compute_pass(&self, writable_textures: &[StorageTextureReadWriteBinding], writable_buffers: &[StorageBufferReadWriteBinding]) -> ComputePass {
        unsafe {
            // SAFETY: pointer casts: `StorageTextureReadWriteBinding` and `StorageBufferReadWriteBinding` are #[repr(transparent)]
            let ptr = SDL_BeginGPUComputePass(self.ptr, writable_textures.as_ptr() as *const _, writable_textures.len() as u32, writable_buffers.as_ptr() as *const _, writable_buffers.len() as u32);
            if ptr.is_null() {
                panic!("compute pass pointer handle should no be nullable");
            }
            ComputePass { ptr }
        }
    }

    pub fn begin_render_pass(&self, color_target_infos: &[ColorTargetInfo]) -> RenderPass {
        unsafe {
            // SAFETY: Pointer conversion: `ColorTargetInfo` is #[repr(transparent)]
            let ptr = SDL_BeginGPURenderPass(self.ptr, color_target_infos.as_ptr() as *const _, color_target_infos.len() as u32, std::ptr::null());
            if ptr.is_null() {
                panic!("GPU render pass pointer should not be nullable")
            }

            RenderPass { ptr }
        }
    }

    pub fn begin_copy_pass(&self) -> CopyPass {
        unsafe {
            let ptr = SDL_BeginGPUCopyPass(self.ptr);
            if ptr.is_null() {
                panic!("GPU copy pass pointer should not be nullable")
            }

            CopyPass { ptr }
        }
    }

    /// Sets the value of the uniform buffer at the given slot binding.
    ///
    /// Make sure to put the uniform in binding set `1`.
    /// Make sure the data layout matches between GLSL and Rust
    /// (Use `#[repr(C)]` or `#[repr(packed)]`).
    /// ```glsl
    /// layout (std430, set = 1, binding = 0) uniform MyUniform { /* ... */ };
    /// ```
    pub fn push_vertex_uniform<T>(&self, binding_index: u32, data: &[T]) {
        unsafe {
            SDL_PushGPUVertexUniformData(self.ptr, binding_index, data.as_ptr() as *const _, (data.len() * std::mem::size_of::<T>()) as u32);
        }
    }

    /// Sets the value of the uniform buffer at the given slot binding.
    /// 
    /// Make sure to put the uniform in binding set `3`.
    /// Make sure the data layout matches between GLSL and Rust
    /// (Use `#[repr(C)]` or `#[repr(packed)]`).
    /// ```glsl
    /// layout (std430, set = 3, binding = 0) uniform MyUniform { /* ... */ };
    /// ```
    pub fn push_fragment_uniform<T>(&self, binding_index: u32, data: &[T]) {
        unsafe {
            SDL_PushGPUFragmentUniformData(self.ptr, binding_index, data.as_ptr() as *const _, (data.len() * std::mem::size_of::<T>()) as u32);
        }
    }

    /// Sets the value of the uniform buffer at the given slot binding.
    /// 
    /// Make sure the data layout matches between GLSL and Rust
    /// (Use `#[repr(C)]` or `#[repr(packed)]`).
    /// ```glsl
    /// layout (std430, set = 2, binding = 0) uniform MyUniform { /* ... */ };
    /// ```
    pub fn push_compute_uniform<T>(&self, binding_index: u32, data: &[T]) {
        unsafe {
            SDL_PushGPUComputeUniformData(self.ptr, binding_index, data.as_ptr() as *const _, (data.len() * std::mem::size_of::<T>()) as u32);
        }
    }

    pub fn submit(self) -> Result<()> {
        unsafe { 
            if !SDL_SubmitGPUCommandBuffer(self.ptr) {
                return Err(ErrorKind::SubmitCommandBuffer.open());
            }
        };
        std::mem::forget(self);
        Ok(())
    }

    pub fn submit_and_acquire_fence<'a>(self, device: &'a Device) -> Result<Fence<'a>> {
        let fence = unsafe { 
            let fence = SDL_SubmitGPUCommandBufferAndAcquireFence(self.ptr);
            if fence.is_null() {
                return Err(ErrorKind::SubmitCommandBuffer.open())
            }
            fence
        };
        std::mem::forget(self);

        Ok(Fence { ptr: fence, device })
    }
}

impl Drop for CommandBuffer {
    fn drop(&mut self) {
        log_warn(LogCategory::GPU, "dropped command buffer without submitting it: call .submit or .submit_and_acquire_fence");
    }
}

impl Fence<'_> {
    /// Blocks until the fence is completed.
    pub fn wait(self) {
        unsafe { 
            SDL_WaitForGPUFences(self.device.ptr, true, &self.ptr as *const _, 1);
        }
    }
}

impl Drop for Fence<'_> {
    fn drop(&mut self) {
        unsafe {
            SDL_ReleaseGPUFence(self.device.ptr, self.ptr);
        }
    }
}

impl CopyPass {
    pub fn end(self) {
        unsafe { SDL_EndGPUCopyPass(self.ptr) }
        std::mem::forget(self);
    }
}

impl Drop for CopyPass {
    fn drop(&mut self) {
        log_warn(LogCategory::GPU, "dropped render pass without ending it");
    }
}

impl ComputePass {
    /// Binds the compute pipeline to be used next in this pass.
    pub fn bind_pipeline(&self, pipeline: &ComputePipeline) {
        unsafe {
            SDL_BindGPUComputePipeline(self.ptr, pipeline.ptr);
        }
    }

    /// Dispatches a compute shader with the currently bound pipeline and state.
    /// 
    /// A VERY IMPORTANT NOTE If you dispatch multiple times in a compute pass, and
    /// the dispatches write to the same resource region as each other, there is no
    /// guarantee of which order the writes will occur. If the write order matters,
    /// you MUST end the compute pass and begin another one.
    /// 
    /// - `groupcount`: number of local workgroups to dispatch in the X, Y, and Z dimension.
    pub fn dispatch(&self, groupcount: [u32; 3]) {
        unsafe {
            SDL_DispatchGPUCompute(self.ptr, groupcount[0], groupcount[1], groupcount[2]);
        }
    }

    /// Binds read only storage buffers.
    /// Theses buffers must have been created with [`TextureUsage::ComputeStorageRead`]
    /// They must be registered in the layout set 0.
    pub fn bind_buffers(&self, first_slot: u32, buffers: &[StorageBufferReadBinding]) {
        unsafe {
            SDL_BindGPUComputeStorageBuffers(self.ptr, first_slot, buffers.as_ptr() as *const _, buffers.len() as u32);
        }
    }

    /// Binds read only storage textures.
    /// Theses textures must have been created with [`TextureUsage::ComputeStorageRead`].
    /// They must be registered in the layout set 0.
    pub fn bind_textures(&self, first_slot: u32, textures: &[&TextureRef]) {
        let mut textures_vec = smallvec::SmallVec::<[*mut SDL_GPUTexture; 8]>::new();
        textures_vec.extend(textures.iter().map(|b| b.ptr));
        unsafe {
            SDL_BindGPUComputeStorageTextures(self.ptr, first_slot, textures_vec.as_ptr(), textures_vec.len() as u32);
        }
    }

    pub fn end(self) {
        unsafe { SDL_EndGPUComputePass(self.ptr) }
        std::mem::forget(self);
    }
}

impl Drop for ComputePass {
    fn drop(&mut self) {
        log_warn(LogCategory::GPU, "dropped compute pass without ending it");
    }
}

impl RenderPass {
    pub fn bind_pipeline(&self, pipeline: &GraphicsPipeline) {
        unsafe { SDL_BindGPUGraphicsPipeline(self.ptr, pipeline.ptr) }
    }

    /// Binds the given vertex buffers to the vertex shader.
    /// Use [`Buffer::vertex_binding`] to create the binding.
    /// 
    /// - `first_slot`: The first binding index at which the vertex buffers will be bound in the shader.
    pub fn bind_vertex_buffer(&self, first_slot: u32, buffers: &[VertexBufferBinding]) {
        unsafe {
            // SAFETY: pointer cast: `BufferBinding` is #[repr(transparent)]
            SDL_BindGPUVertexBuffers(self.ptr, first_slot, buffers.as_ptr() as *const _, buffers.len() as u32);
        }
    }

    /// Binds read only storage buffers to the vertex shader.
    /// The buffers must have been created with [`BufferUsage::GraphicsStorageRead`].
    /// Use [`Buffer::read_binding`] to create the binding.
    /// - `first_slot`: The first binding index at which the buffers will be bound in the shader.
    pub fn bind_vertex_storage_buffers(&self, first_slot: u32, buffers: &[StorageBufferReadBinding]) {
        unsafe {
            SDL_BindGPUVertexStorageBuffers(self.ptr, first_slot, buffers.as_ptr() as *const _, buffers.len() as u32);
        }
    }

    /// Binds read only storage buffers to the fragment shader.
    /// The buffers must have been created with [`BufferUsage::GraphicsStorageRead`].
    /// Use [`Buffer::read_binding`] to create the binding.
    /// - `first_slot`: The first binding index at which the buffers will be bound in the shader.
    pub fn bind_fragment_storage_buffers(&self, first_slot: u32, buffers: &[StorageBufferReadBinding]) {
        unsafe {
            SDL_BindGPUFragmentStorageBuffers(self.ptr, first_slot, buffers.as_ptr() as *const _, buffers.len() as u32);
        }
    }

    /// Draw GPU primitives using the currently bounds graphics pipeline,
    /// vertex buffers, texture samplers, storage buffers and storage textures.
    /// 
    /// Note that the `first_vertex` and `first_instance` parameters are NOT
    /// compatible with built-in vertex/instance ID variables in shaders (for
    /// example, SV_VertexID). If your shader depends on these variables, the
    /// correlating draw call parameter MUST be 0.
    pub fn draw_primitives(&self, num_vertices: usize, num_instances: usize, first_vertex: usize, first_instance: usize) {
        unsafe {
            SDL_DrawGPUPrimitives(self.ptr, num_vertices as u32, num_instances as u32, first_vertex as u32, first_instance as u32);
        }
    }

    pub fn end(self) {
        unsafe { SDL_EndGPURenderPass(self.ptr) }
        std::mem::forget(self);
    }
}

impl Drop for RenderPass {
    fn drop(&mut self) {
        log_warn(LogCategory::GPU, "dropped render pass without ending it");
    }
}
