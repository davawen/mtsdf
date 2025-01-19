use std::ffi::CStr;

use sdl3_sys::gpu::*;
use bitflags::bitflags;

use crate::error::ErrorKind;

use super::{Device, Result};

bitflags! {
    #[derive(Clone, Copy)]
    pub struct ShaderFormat: u32 {
        const Invalid = SDL_GPU_SHADERFORMAT_INVALID;
        const Private = SDL_GPU_SHADERFORMAT_PRIVATE;
        const Spirv = SDL_GPU_SHADERFORMAT_SPIRV;
        const Dxbc = SDL_GPU_SHADERFORMAT_DXBC;
        const Dxil = SDL_GPU_SHADERFORMAT_DXIL;
        const Msl = SDL_GPU_SHADERFORMAT_MSL;
        const Metallib = SDL_GPU_SHADERFORMAT_METALLIB;
    }
}

pub type ShaderStage = SDL_GPUShaderStage;

pub struct Shader<'a> {
    device: &'a Device,
    pub ptr: *mut SDL_GPUShader
}

#[derive(Clone, Copy)]
pub struct ShaderCreate<'a> {
    /// The bytecode format used by the shader
    /// This needs to change depending on the backend you are using
    pub format: ShaderFormat,
    pub stage: ShaderStage,
    /// The main shader function
    /// Default value is `"main"`.
    /// NOTE: You can use C string literals to easily define a value
    pub entrypoint: &'a CStr,
    /// The number of texture samplers used in the shader
    /// With SPIR-V bytecode, samplers should be put in ressource set:
    /// - 0 for vertex shaders 
    /// - 2 for fragment shaders
    /// That is, in glsl:
    /// ```glsl
    /// layout (set = 0, binding = 0) uniform sampler2D my_texture;
    /// ```
    /// 
    pub num_samplers: u32,
    /// The number of storage textures used in the shader
    /// With SPIR-V bytecode, textures should be put in ressource set:
    /// - 0 for vertex shaders 
    /// - 2 for fragment shaders
    /// That is, in glsl:
    /// ```glsl
    /// layout (set = 0, binding = 0, rgba32f) uniform image2D my_texture;
    /// ```
    /// 
    pub num_storage_textures: u32,
    /// The number of storage buffers used in the shader
    /// With SPIR-V bytecode, buffers should be put in ressource set:
    /// - 0 for vertex shaders 
    /// - 2 for fragment shaders
    /// That is, in glsl:
    /// ```glsl
    /// layout (std430, set = 0, binding = 0) buffer MyBuffer {
    ///     float values[];
    /// };
    /// ```
    /// 
    pub num_storage_buffers: u32,
    /// The number of uniform buffers used in the shader
    /// With SPIR-V bytecode, uniform buffers should be put in ressource set:
    /// - 1 for vertex shaders 
    /// - 3 for fragment shaders
    /// That is, in glsl:
    /// ```glsl
    /// // In the vertex stage:
    /// layout (set = 1, location = 0) uniform MyUniform { float value; };
    /// // In the fragment stage:
    /// layout (set = 3, location = 0) uniform MyUniform { float value; };
    /// ```
    /// 
    pub num_uniform_buffers: u32
}

impl Default for ShaderCreate<'_> {
    fn default() -> Self {
        Self {
            stage: ShaderStage::VERTEX,
            format: ShaderFormat::Spirv,
            entrypoint: c"main",
            num_samplers: 0,
            num_storage_buffers: 0,
            num_storage_textures: 0,
            num_uniform_buffers: 0
        }
    }
}

impl<'a> Shader<'a> {
    /// Creates a new shader with the given parameters. 
    pub fn new(device: &'a Device, code: &[u8], params: ShaderCreate) -> Result<Self> {
        let mut info = SDL_GPUShaderCreateInfo {
            code: code.as_ptr(),
            code_size: code.len(),
            format: params.format.bits(),
            entrypoint: params.entrypoint.as_ptr(),
            num_samplers: params.num_samplers,
            num_storage_textures: params.num_storage_textures,
            num_storage_buffers: params.num_storage_buffers,
            num_uniform_buffers: params.num_uniform_buffers,
            stage: params.stage,
            props: 0
        };

        unsafe {
            let ptr = SDL_CreateGPUShader(device.ptr, &mut info as *mut _);
            if ptr.is_null() {
                return Err(ErrorKind::ShaderCreation.open())
            }
            Ok(Shader { device, ptr })
        }
    }
}

impl Drop for Shader<'_> {
    fn drop(&mut self) {
        unsafe {
            SDL_ReleaseGPUShader(self.device.ptr, self.ptr);
        }
    }
}
