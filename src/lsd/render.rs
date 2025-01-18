use sdl3_sys::render::{SDL_CreateRenderer, SDL_Renderer};

use crate::{error::{ErrorKind, Result}, Window};

pub struct Renderer {
    pub ptr: *mut SDL_Renderer
}

pub fn create_renderer(window: &Window) -> Result<Renderer> {
    // let cname = CString::new(name).unwrap();
    unsafe {
        let ptr = SDL_CreateRenderer(window.ptr, std::ptr::null());
        if ptr.is_null() {
            return Err(ErrorKind::RendererCreation.open());
        }
        Ok(Renderer { ptr })
    }
}

