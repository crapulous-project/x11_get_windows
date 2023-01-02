use std::ffi::c_int;
use std::os::raw::c_long;

use x11::xlib::{ButtonPress, ButtonPressMask, ButtonRelease, ButtonReleaseMask};

pub enum ButtonType {
	Press,
	Release,
}

impl ButtonType {
	/// Get type mask
	#[inline]
	pub fn mask(&self) -> c_long {
		match self {
			ButtonType::Press => {
				ButtonPressMask
			}
			ButtonType::Release => {
				ButtonReleaseMask
			}
		}
	}
}

impl From<ButtonType> for c_int {
	fn from(value: ButtonType) -> Self {
		match value {
			ButtonType::Press => {
				ButtonPress
			}
			ButtonType::Release => {
				ButtonRelease
			}
		}
	}
}