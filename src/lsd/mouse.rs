use sdl3_sys::mouse::SDL_GetMouseState;

pub fn get_mouse_pos() -> (f32, f32) {
    let mut x = 0.0;
    let mut y = 0.0;
    unsafe {
        let _ = SDL_GetMouseState(&raw mut x, &raw mut y);
    }
    (x, y)
}
