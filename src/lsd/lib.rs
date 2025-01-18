use std::{ffi::CString, mem::MaybeUninit, time::Duration};

use sdl3_sys::{events::{SDL_Event, SDL_PollEvent}, init::*, pixels::SDL_FColor, timer::SDL_DelayNS, video::{SDL_CreateWindow, SDL_Window, SDL_WindowFlags}};

pub mod gpu;
pub mod error;
pub mod log;
pub mod render;

pub type Color = SDL_FColor;

use error::{ErrorKind, Result};

use bitflags::bitflags;

bitflags! {
    pub struct InitFlags: u32 {
        const Audio = SDL_INIT_AUDIO;
        const Video = SDL_INIT_VIDEO;
        const Joystick = SDL_INIT_JOYSTICK;
        const Haptic = SDL_INIT_HAPTIC;
        const Gamepad = SDL_INIT_GAMEPAD;
        const Events = SDL_INIT_EVENTS;
        const Sensor = SDL_INIT_SENSOR;
        const Camera = SDL_INIT_CAMERA;
    }

    pub struct WindowFlags: u64 {
        /// window is in fullscreen mode
        const Fullscreen = 1_u64;
        /// window usable with OpenGL context
        const Opengl = 2_u64;
        /// window is occluded
        const Occluded = 4_u64;
        /// window is neither mapped onto the desktop nor shown in the taskbar/dock/window list; [`SDL_ShowWindow()`] is required for it to become visible
        const Hidden = 8_u64;
        /// no window decoration
        const Borderless = 16_u64;
        /// window can be resized
        const Resizable = 32_u64;
        /// window is minimized
        const Minimized = 64_u64;
        /// window is maximized
        const Maximized = 128_u64;
        /// window has grabbed mouse input
        const MouseGrabbed = 256_u64;
        /// window has input focus
        const InputFocus = 512_u64;
        /// window has mouse focus
        const MouseFocus = 1024_u64;
        /// window not created by SDL
        const External = 2048_u64;
        /// window is modal
        const Modal = 4096_u64;
        /// window uses high pixel density back buffer if possible
        const HighPixelDensity = 8192_u64;
        /// window has mouse captured (unrelated to MOUSE_GRABBED)
        const MouseCapture = 16384_u64;
        /// window has relative mode enabled
        const MouseRelativeMode = 32768_u64;
        /// window should always be above others
        const AlwaysOnTop = 65536_u64;
        /// window should be treated as a utility window, not showing in the task bar and window list
        const Utility = 131072_u64;
        /// window should be treated as a tooltip and does not get mouse or keyboard focus, requires a parent window
        const Tooltip = 262144_u64;
        /// window should be treated as a popup menu, requires a parent window
        const PopupMenu = 524288_u64;
        /// window has grabbed keyboard input
        const KeyboardGrabbed = 1048576_u64;
        /// window usable for Vulkan surface
        const Vulkan = 268435456_u64;
        /// window usable for Metal view
        const Metal = 536870912_u64;
        /// window with transparent buffer
        const Transparent = 1073741824_u64;
        /// window should not be focusable
        const NotFocusable = 2147483648_u64;
    }
}

pub struct SDL;

pub fn init(flags: InitFlags) -> Result<SDL> {
    unsafe { 
        if !SDL_Init(flags.bits()) {
            return Err(ErrorKind::Init.open())
        }
    }

    Ok(SDL)
}

impl Drop for SDL {
    fn drop(&mut self) {
        unsafe { SDL_Quit() }
    }
}

pub struct Window {
    pub ptr: *mut SDL_Window
}

pub fn create_window(_: &SDL, name: &str, width: i32, height: i32, flags: WindowFlags) -> Result<Window> {
    let cname = CString::new(name).unwrap();
    unsafe { 
        let ptr = SDL_CreateWindow(cname.as_ptr(), width, height, flags.bits());
        if ptr.is_null() {
            return Err(ErrorKind::WindowCreation.open());
        }
        Ok(Window { ptr })
    }
}

pub fn poll_event() -> Option<SDL_Event> {
    let mut event: MaybeUninit<SDL_Event> = MaybeUninit::uninit();

    unsafe { 
        if SDL_PollEvent(event.as_mut_ptr()) {
            Some(event.assume_init())
        } else {
            None
        }
    }
}

pub fn delay(duration: Duration) {
    let time = duration.as_nanos();
    unsafe { SDL_DelayNS(time as u64) }
}
