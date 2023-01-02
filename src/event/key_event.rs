use std::ffi::c_int;
use std::os::raw::c_long;

use x11::xlib::{KeyPress, KeyPressMask, KeyRelease, KeyReleaseMask};

/// Key event type
pub enum KeyType {
	/// Key press
	Press,
	/// Key release
	Release,
}

impl KeyType {
	/// Get type mask
	#[inline]
	pub fn mask(&self) -> c_long {
		match self {
			KeyType::Press => {
				KeyPressMask
			}
			KeyType::Release => {
				KeyReleaseMask
			}
		}
	}
}

impl From<KeyType> for c_int {
	fn from(value: KeyType) -> Self {
		match value {
			KeyType::Press => {
				KeyPress
			}
			KeyType::Release => {
				KeyRelease
			}
		}
	}
}