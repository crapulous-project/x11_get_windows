use std::{
	ffi::CStr,
	ops::Drop,
	os::raw::c_void,
	ptr::null_mut,
	slice,
};
use std::borrow::BorrowMut;
use std::ops::Deref;
use std::rc::Rc;
use std::slice::Windows;

use x11::xlib::{Window as XWindow, XA_WINDOW, XAllPlanes, XDefaultRootWindow, XFree, XGetImage, XGetWindowAttributes, XGetWMName, XImage, XTextProperty, XWindowAttributes};
use x11::xlib;

use crate::{
	Display,
	NotSupported,
	Null,
	Session,
	util::get_window_property,
};

/// This struct represents a window and holds the ID of that window that can be used
/// to query for its name.
pub struct Window(pub XWindow, pub Rc<Display>);

impl Clone for Window {
	fn clone(&self) -> Self {
		Self(self.0, Rc::clone(&self.1))
	}
}

impl Window {
	/// Gets the default root window of a display.
	///
	/// A wrapper around the [XDefaultRootWindow] function.
	pub fn default_root_window(display: Rc<Display>) -> Self {
		let win = unsafe { XDefaultRootWindow(display.0) };
		Window(win, display)
	}
	/// Gets the current active window.
	///
	/// This function uses a [Session] struct and will update the properties
	/// that are set to [None] but are required.
	/// This uses the display, root_window, and active_window_atom properties
	/// of the [Session] struct.
	pub fn active_window(session: &Session) -> Result<Self, NotSupported> {
		let Session { display, .. } = session;
		let root_window = session.root().clone();
		let active_window_atom = session.active_list();
		let response = unsafe { get_window_property(display, root_window, *active_window_atom, XA_WINDOW)? };
		let window = match response.actual_format_return {
			8 => {
				unsafe { slice::from_raw_parts(response.proper_return as *const u8, response.nitems_return as usize) }
					.first()
					.map(|x| Window(*x as XWindow, Rc::clone(display)))
			}
			16 => {
				unsafe { slice::from_raw_parts(response.proper_return as *const u16, response.nitems_return as usize) }
					.first()
					.map(|x| Window(*x as XWindow, Rc::clone(display)))
			}
			32 => {
				unsafe { slice::from_raw_parts(response.proper_return as *const usize, response.nitems_return as usize) }
					.first()
					.map(|x| Window(*x as XWindow, Rc::clone(display)))
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
	pub fn get_title(&self, display: &Display) -> Result<WindowTitle, Null> {
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

	pub(crate) fn match_title(&self, title: impl AsRef<[u8]>) -> bool {
		let mut text_property = XTextProperty {
			value: null_mut(),
			encoding: 0,
			format: 0,
			nitems: 0,
		};
		unsafe {
			XGetWMName(
				self.1.0,
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

	/// Get window attribute
	pub fn get_attr(&self, display: &Display) -> XWindowAttributes {
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
		attr
	}

	/// Capture screenshot of this window
	pub fn capture(&self, display: &Display) -> XImg {
		let attr = self.get_attr(display);
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

/// This struct represent pixel value from XImage
#[repr(C, align(4))]
pub struct XColor {
	/// Blue value of current pixel
	pub b: u8,
	/// Green value of current pixel
	pub g: u8,
	/// Red value of current pixel
	pub r: u8,
	_pad: u8,
}

impl XColor {
	/// Get gray scale value by sum RGB and divide by 3
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
		let len = (self.height() * self.width()) as usize;
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
}

impl AsRef<XImage> for XImg {
	#[inline]
	fn as_ref(&self) -> &XImage { unsafe { self.img.as_ref() }.unwrap() }
}

impl AsMut<XImage> for XImg {
	#[inline]
	fn as_mut(&mut self) -> &mut XImage {
		unsafe { self.img.as_mut() }.unwrap()
	}
}

impl Drop for XImg {
	fn drop(&mut self) {
		unsafe { XFree(self.img as _); }
	}
}