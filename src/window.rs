use std::{
	ffi::CStr,
	ops::Drop,
	os::raw::c_void,
	ptr::null_mut,
	slice,
};
use std::borrow::BorrowMut;
use std::ops::Deref;
use std::slice::Windows;

use x11::xlib::{Window as XWindow, XA_WINDOW, XAllPlanes, XDefaultRootWindow, XFree, XGetImage, XGetWindowAttributes, XGetWMName, XImage, XTextProperty, XWindowAttributes};
use x11::xlib;

use crate::{
	Atom,
	Display,
	NET_ACTIVE_WINDOW,
	NotSupported,
	Null,
	Session,
	util::get_window_property,
};

/// This struct represents a window and holds the ID of that window that can be used
/// to query for its name.
#[derive(Copy, Clone, Debug)]
pub struct Window(pub XWindow);

impl Window {
	/// Gets the default root window of a display.
	///
	/// A wrapper around the [XDefaultRootWindow] function.
	pub fn default_root_window(display: &Display) -> Self {
		let win = unsafe { XDefaultRootWindow(display.0) };
		Window(win)
	}
	/// Gets the current active window.
	///
	/// This function uses a [Session] struct and will update the properties
	/// that are set to [None] but are required.
	/// This uses the display, root_window, and active_window_atom properties
	/// of the [Session] struct.
	pub fn active_window(session: &mut Session) -> Result<Self, NotSupported> {
		let Session { display, root_window, active_window_atom, .. } = session;
		let root_window = root_window.get_or_insert_with(|| Window::default_root_window(display));
		let active_window_atom = active_window_atom.get_or_insert_with(|| Atom::new(display, NET_ACTIVE_WINDOW).unwrap());
		let response = unsafe { get_window_property(display, *root_window, *active_window_atom, XA_WINDOW)? };
		let window = match response.actual_format_return {
			8 => {
				unsafe { slice::from_raw_parts(response.proper_return as *const u8, response.nitems_return as usize) }
					.first()
					.map(|x| Window(*x as XWindow))
			}
			16 => {
				unsafe { slice::from_raw_parts(response.proper_return as *const u16, response.nitems_return as usize) }
					.first()
					.map(|x| Window(*x as XWindow))
			}
			32 => {
				unsafe { slice::from_raw_parts(response.proper_return as *const usize, response.nitems_return as usize) }
					.first()
					.map(|x| Window(*x as XWindow))
			}
			_ => { None }
		};
		unsafe { XFree(response.proper_return as *mut c_void) };
		window.ok_or(NotSupported)
	}
	/// Gets the title of the window.
	///
	/// If the window does not have a title, a null pointer may be returned.
	/// In that case the [Null] error is returned.
	/// However, I have not encountered a [Null] error yet.
	pub fn get_title(self, display: &Display) -> Result<WindowTitle, Null> {
		let mut text_property = XTextProperty {
			value: null_mut(),
			encoding: 0,
			format: 0,
			nitems: 0,
		};
		unsafe {
			XGetWMName(
				display.0,
				self.0,
				&mut text_property,
			)
		};
		if !text_property.value.is_null() {
			let text = unsafe { CStr::from_ptr(text_property.value as *mut i8) };
			Ok(WindowTitle(text))
		} else { Err(Null) }
	}

	pub(crate) fn match_title(self, display: &Display, title: impl AsRef<[u8]>) -> bool {
		let mut text_property = XTextProperty {
			value: null_mut(),
			encoding: 0,
			format: 0,
			nitems: 0,
		};
		unsafe {
			XGetWMName(
				display.0,
				self.0,
				&mut text_property,
			)
		};
		if !text_property.value.is_null() {
			let text = unsafe { CStr::from_ptr(text_property.value as *mut i8) };
			text.to_bytes() == title.as_ref()
		} else {
			false
		}
	}

	/// Capture screenshot of this window
	pub fn capture(&self, display: &Display) -> XImg {
		let mut attr = XWindowAttributes {
			x: 0,
			y: 0,
			width: 0,
			height: 0,
			border_width: 0,
			depth: 0,
			visual: null_mut(),
			root: 0,
			class: 0,
			bit_gravity: 0,
			win_gravity: 0,
			backing_store: 0,
			backing_planes: 0,
			backing_pixel: 0,
			save_under: 0,
			colormap: 0,
			map_installed: 0,
			map_state: 0,
			all_event_masks: 0,
			your_event_mask: 0,
			do_not_propagate_mask: 0,
			override_redirect: 0,
			screen: null_mut(),
		};
		unsafe { XGetWindowAttributes(display.0, self.0, attr.borrow_mut() as _) };

		let width = attr.width as u32;
		let height = attr.height as u32;

		let img = unsafe { XGetImage(display.0, self.0, 0, 0, width, height, XAllPlanes(), xlib::ZPixmap) };
		XImg { img }
	}
}

#[derive(Debug)]
pub struct WindowTitle<'a>(&'a CStr);

impl<'a> AsRef<CStr> for WindowTitle<'a> {
	fn as_ref(&self) -> &CStr {
		self.0
	}
}

impl WindowTitle<'_> {
	fn as_bytes(&self) -> &[u8] {
		self.0.to_bytes()
	}
}

impl<'a> Drop for WindowTitle<'a> {
	fn drop(&mut self) {
		unsafe { XFree(self.0.as_ptr() as *mut c_void) };
	}
}

/// BGRA image format
///
/// XFree is handled by dropping this struct
pub struct XImg {
	img: *mut XImage,
}

#[repr(C, align(4))]
pub struct XColor {
	pub b: u8,
	pub g: u8,
	pub r: u8,
	_pad: u8,
}

impl XColor {
	pub fn grayscale_approx(&self) -> u8 {
		((self.b as u16 + self.g as u16 + self.r as u16) / 3) as u8
	}

	/// ref: https://go.dev/src/image/color/color.go#:~:text=(19595*r%20%2B%2038470*g%20%2B%207471*b%20%2B%201%3C%3C15)%20%3E%3E%2024
	pub fn grayscale(&self) -> u8 {
		let r = self.r as u32;
		let g = self.g as u32;
		let b = self.b as u32;
		(((19595 * r + 38470 * g + 7471 * b) + (1 << 15)) >> 16) as u8
	}
}

impl Deref for XImg {
	type Target = [XColor];
	#[inline]
	fn deref(&self) -> &[XColor] {
		let len = ((self.height() * self.width()) as usize);
		unsafe { slice::from_raw_parts((*self.img).data as _, len) }
	}
}

impl XImg {
	/// Get image width
	#[inline]
	pub fn width(&self) -> u32 { self.as_ref().width as u32 }

	/// Get image height
	#[inline]
	pub fn height(&self) -> u32 { self.as_ref().height as u32 }

	/// Get image color slices by row
	#[inline]
	pub fn rows(&self) -> Windows<'_, XColor> { self.deref().windows(self.width() as _) }

	/// Get raw image pointer
	#[inline]
	pub fn as_ptr(&self) -> *mut XImage { self.img }

	/// Get image reference
	#[inline]
	pub fn as_ref(&self) -> &XImage { unsafe { self.img.as_ref() }.unwrap() }

	/// Get mutable image reference
	#[inline]
	pub fn as_mut(&mut self) -> &mut XImage { unsafe { self.img.as_mut() }.unwrap() }
}

impl Drop for XImg {
	fn drop(&mut self) {
		unsafe { XFree(self.img as _); }
	}
}