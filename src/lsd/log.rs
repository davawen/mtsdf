use std::ffi::CString;

use sdl3_sys::log::{SDL_LogCategory, SDL_LogMessage, SDL_LogPriority};

pub type LogPriority = SDL_LogPriority;
pub type LogCategory = SDL_LogCategory;

pub fn log_message(category: LogCategory, priority: LogPriority, msg: &str) {
    let cmsg = CString::new(msg).unwrap();
    unsafe {
        SDL_LogMessage(category.0, priority, cmsg.as_ptr());
    }
}

pub fn log_trace(category: LogCategory, msg: &str) {
    log_message(category, LogPriority::TRACE, msg);
}

pub fn log_verbose(category: LogCategory, msg: &str) {
    log_message(category, LogPriority::VERBOSE, msg);
}

pub fn log_debug(category: LogCategory, msg: &str) {
    log_message(category, LogPriority::DEBUG, msg);
}

pub fn log_info(category: LogCategory, msg: &str) {
    log_message(category, LogPriority::INFO, msg);
}

pub fn log_warn(category: LogCategory, msg: &str) {
    log_message(category, LogPriority::WARN, msg);
}

pub fn log_error(category: LogCategory, msg: &str) {
    log_message(category, LogPriority::ERROR, msg);
}

pub fn log_critical(category: LogCategory, msg: &str) {
    log_message(category, LogPriority::CRITICAL, msg);
}
