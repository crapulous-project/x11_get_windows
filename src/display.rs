use x11::xlib::{
    Display as XDisplay,
    XOpenDisplay,
    XCloseDisplay,
};
use std::{
    ops::Drop,
    ptr::null,
};
use std::rc::Rc;

/// The Display Struct is just a wrapper of a [*mut Display] from XLib.
/// 
/// When this struct is dropped, the reference will be dropped using [XCloseDisplay].
pub struct Display(pub *mut XDisplay);
impl Display {
    /// Opens a connection to the x11 server.
    /// 
    /// Will return an error of [Null] if the returned Display pointer is a null pointer.
    pub fn open() -> Option<Self> {
        let x_display = unsafe { XOpenDisplay( null() ) };
        if x_display.is_null() {
            return None
        }
        Some(Display(x_display))
    }

    /// Create [Rc] for sharing in internal lib
    pub fn shared(self) -> Rc<Self> {
        Rc::new(self)
    }
    
    /// Consumes the safe wrapper and returns a pointer to the raw Display.
    /// 
    /// Use this if you want to get more out of the display that this crate cannot provide.
    /// # Safety
    /// this is safe operation, but you will have to free manually or use [Self::from_raw] to use automatic destructor
    pub unsafe fn into_raw(self) -> *mut XDisplay {
        self.0
    }
    /// Wraps a raw display pointer with a safe wrapper.
    /// 
    /// Ensure that this pointer is the only pointer as the connection is closed when this struct is dropped.
    /// # Safety
    /// this is safe operation as long as you didn't construct [Display] using [Display::from_raw] multiple time
    pub unsafe fn from_raw(display: *mut XDisplay) -> Self {
        Display(display)
    }
}
impl Drop for Display {
    fn drop(&mut self) {
        unsafe { XCloseDisplay(self.0) };
    }
}
