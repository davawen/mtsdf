use std::ffi::CStr;

use sdl3_sys::error::SDL_GetError;

#[derive(Clone, Debug)]
pub enum ErrorKind {
    Init,
    WindowCreation,
    RendererCreation,
    GpuDeviceCreation,
    ShaderCreation,
    BufferCreation,
    TextureCreation,
    TransferBufferCreation,
    TransferBufferMap,
    GraphicsPipelineCreation,
    ComputePipelineCreation,
    AcquireCommandBuffer,
    SubmitCommandBuffer,
    AcquireSwapchainTexture,
    Str(String)
}

impl ErrorKind {
    pub fn new(message: impl Into<String>) -> Error {
        ErrorKind::Str(message.into()).open()
    }

    pub fn open(self) -> Error {
        let sdl_error = unsafe { SDL_GetError() };
        if sdl_error.is_null() {
            Error { kind: self, sdl_error: String::new() }
        } else {
            Error {
                kind: self,
                sdl_error: unsafe { CStr::from_ptr(sdl_error).to_string_lossy().to_string() }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Error {
    kind: ErrorKind,
    sdl_error: String
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "error: ")?;
        match &self.kind {
            ErrorKind::Init => writeln!(f, "failed to initialize SDL:")?,
            ErrorKind::WindowCreation => writeln!(f, "failed to create window:")?,
            ErrorKind::RendererCreation => writeln!(f, "failed to create renderer:")?,
            ErrorKind::GpuDeviceCreation => writeln!(f, "failed to create gpu device:")?,
            ErrorKind::ShaderCreation => writeln!(f, "failed to create shader:")?,
            ErrorKind::BufferCreation => writeln!(f, "failed to create buffer:")?,
            ErrorKind::TextureCreation => writeln!(f, "failed to create texture:")?,
            ErrorKind::TransferBufferCreation => writeln!(f, "failed to create transfer buffer:")?,
            ErrorKind::TransferBufferMap => writeln!(f, "failed to map transfer buffer to memory:")?,
            ErrorKind::GraphicsPipelineCreation => writeln!(f, "failed to create graphics pipeline:")?,
            ErrorKind::ComputePipelineCreation => writeln!(f, "failed to create compute pipeline:")?,
            ErrorKind::AcquireCommandBuffer => writeln!(f, "failed to acquire gpu command buffer:")?,
            ErrorKind::AcquireSwapchainTexture => writeln!(f, "failed to acquire gpu swapchain texture:")?,
            ErrorKind::SubmitCommandBuffer => writeln!(f, "failed to submit gpu command buffer:")?,
            ErrorKind::Str(s) => writeln!(f, "{s}:")?
        }
        writeln!(f, "{}", self.sdl_error)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

