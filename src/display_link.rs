use std::ffi::c_void;

pub enum CVDisplayLink {}

pub type DisplayLinkOutputCallback = unsafe extern "C" fn(
    display_link: *mut CVDisplayLink,
    current_time: *const c_void,
    output_time: *const c_void,
    flags_in: i64,
    flags_out: *mut i64,
    user_info: *mut c_void,
) -> i32;

pub struct DisplayLink {
    raw: *mut CVDisplayLink,
}

impl DisplayLink {
    pub fn new(
        callback: DisplayLinkOutputCallback,
        user_info: *mut c_void,
    ) -> Result<Self, String> {
        let mut raw = std::ptr::null_mut();
        let code = unsafe { CVDisplayLinkCreateWithActiveCGDisplays(&mut raw) };
        if code != 0 {
            return Err(format!("could not create display link, code: {code}"));
        }

        let code = unsafe { CVDisplayLinkSetOutputCallback(raw, callback, user_info) };
        if code != 0 {
            unsafe {
                CVDisplayLinkRelease(raw);
            }
            return Err(format!("could not set display link callback, code: {code}"));
        }

        Ok(Self { raw })
    }

    pub fn start(&mut self) -> Result<(), String> {
        let code = unsafe { CVDisplayLinkStart(self.raw) };
        if code != 0 {
            return Err(format!("could not start display link, code: {code}"));
        }
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), String> {
        let code = unsafe { CVDisplayLinkStop(self.raw) };
        if code != 0 {
            return Err(format!("could not stop display link, code: {code}"));
        }
        Ok(())
    }
}

impl Drop for DisplayLink {
    fn drop(&mut self) {
        let _ = self.stop();
        unsafe {
            CVDisplayLinkRelease(self.raw);
        }
    }
}

#[link(name = "CoreVideo", kind = "framework")]
unsafe extern "C" {
    fn CVDisplayLinkCreateWithActiveCGDisplays(display_link_out: *mut *mut CVDisplayLink) -> i32;
    fn CVDisplayLinkSetOutputCallback(
        display_link: *mut CVDisplayLink,
        callback: DisplayLinkOutputCallback,
        user_info: *mut c_void,
    ) -> i32;
    fn CVDisplayLinkStart(display_link: *mut CVDisplayLink) -> i32;
    fn CVDisplayLinkStop(display_link: *mut CVDisplayLink) -> i32;
    fn CVDisplayLinkRelease(display_link: *mut CVDisplayLink);
}
