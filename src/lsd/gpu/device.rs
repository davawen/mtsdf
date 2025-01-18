use sdl3_sys::gpu::*;

use super::*;

pub struct Device {
    pub ptr: *mut SDL_GPUDevice
}

impl Drop for Device {
    fn drop(&mut self) {
        if self.ptr.is_null() { return }

        // FIXME: Destroy the gpu device
        // For now, vulkan validation layer complain that objects were not destroyed,
        // even though SDL_Release* functions were called on them.
        unsafe { 
            // SDL_DestroyGPUDevice(self.ptr);
        }
    }
}

impl Device {
    /// `backend_name` refers to a specific GPU backend (like "vulkan", "DX11", etc...)
    pub fn new(format: ShaderFormat, debug_mode: bool, backend_name: Option<&str>) -> Result<Device> {
        let backend_name = backend_name.map(|s| CString::new(s).unwrap());

        unsafe {
            let ptr = SDL_CreateGPUDevice(format.bits(), debug_mode, backend_name.as_ref().map(|s| s.as_ptr()).unwrap_or(std::ptr::null()));
            if ptr.is_null() {
                return Err(ErrorKind::GpuDeviceCreation.open())
            }
            Ok(Device { ptr })
        }
    }

    /// Claims a window, creating a swapchain texture for it.
    ///
    /// You must call this function before doing anything with the window
    /// using the GPU module.
    pub fn claim_window(&self, window: &Window) -> Result<()> {
        unsafe {
            if !SDL_ClaimWindowForGPUDevice(self.ptr, window.ptr) {
                return Err(ErrorKind::new("failed to claim window for gpu device"));
            }
        }
        Ok(())
    }

    // pub fn create_graphics_pipeline(&self) -> Result<GraphicsPipeline> {
    //     unsafe {
    //         let create_info = SDL_GPUGraphicsPipelineCreateInfo {
    //             
    //         };
    //     }
    // }

    /// 
    pub fn acquire_command_buffer(&self) -> Result<CommandBuffer> {
        unsafe {
            let ptr = SDL_AcquireGPUCommandBuffer(self.ptr);
            if ptr.is_null() {
                return Err(ErrorKind::AcquireCommandBuffer.open());
            }
            Ok(CommandBuffer { ptr })
        }
    }

    pub fn swapchain_texture_format(&self, window: &Window) -> TextureFormat {
        unsafe { SDL_GetGPUSwapchainTextureFormat(self.ptr, window.ptr) }
    }
}

