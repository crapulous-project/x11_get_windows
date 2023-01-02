use std::{
	os::raw::c_void,
	slice,
};
use std::rc::Rc;
use std::sync::RwLock;

use x11::xlib::{
	Window as XWindow,
	XA_WINDOW,
	XFree,
};

use crate::{Atom, Display, NET_ACTIVE_WINDOW, NET_CLIENT_LIST, NotSupported, util::{
	get_window_property,
	GetWindowPropertyResponse,
}, Window, Windows};
use crate::util::RwLockCell;

/// This is meant to be a struct that makes it easy to use this crate.
///
/// This is purely for convenience.
///
/// # Example
/// ```ignore
/// let mut session = Session::open()
///    .expect("Error opening a new session.");
/// session
///    .get_windows()
///    .expect("Could not get a list of windows.")
///    .iter()
///    .filter_map(|x| x.get_title(&session.display).ok())
///    .for_each(|x| println!("{:?}", x.as_ref()))
/// // Prints out the title for every window that is visible on the screen.
/// ```
pub struct Session {
	/// A display that has been opened.
	pub display: Rc<Display>,
	/// The root window of the display.
	root_window: RwLock<Option<Window>>,
	/// The atom that represents the client_list property.
	client_list_atom: RwLock<Option<Atom>>,
	/// The atom that represents the active_window property.
	pub active_window_atom: RwLock<Option<Atom>>,
}

impl Session {
	/// Opens a display.
	pub fn open() -> Option<Self> {
		Some(Self {
			display: Rc::new(Display::open()?),
			root_window: RwLock::new(None),
			client_list_atom: RwLock::new(None),
			active_window_atom: RwLock::new(None),
		})
	}
	/// Creates a session from an already opened Display connection.
	///
	/// See [Display::open] for more information.
	pub fn from_display(display: Display) -> Self {
		Self {
			display: display.shared(),
			root_window: RwLock::new(None),
			client_list_atom: RwLock::new(None),
			active_window_atom: RwLock::new(None),
		}
	}

	/// Get root windows of this session
	pub fn root(&self) -> &Window {
		self.root_window.get_or_insert_with(|| Window::default_root_window(Rc::clone(&self.display)))
	}
	
	/// Get client list window atom of this session
	pub fn client_list(&self) -> &Atom {
		self.client_list_atom.get_or_insert_with(|| Atom::new(&self.display, NET_CLIENT_LIST).unwrap())
	}
	
	/// Get client list active window atom of this session
	pub fn active_list(&self) -> &Atom {
		self.active_window_atom.get_or_insert_with(|| Atom::new(&self.display, NET_ACTIVE_WINDOW).unwrap())
	}

	/// Gets all the current windows on the screen.
	///
	/// This will update any values that are set to [None] if it needs to use them.
	///
	/// This can possible produce a [NotSupported] error.
	/// In that case, please read the documentation for that struct.
	pub fn get_windows(&self) -> Result<Windows, NotSupported> {
		let Session { display, .. } = self;
		let root = self.root();
		let atom = self.client_list();

		let GetWindowPropertyResponse {
			actual_type_return: return_type,
			actual_format_return: return_format,
			nitems_return: return_nitems,
			proper_return: return_proper,
			..
		} = unsafe { get_window_property(display, root.clone(), *atom, XA_WINDOW)? };
		if return_type == XA_WINDOW {
			let windows = match return_format {
				8 => {
					let array = unsafe { slice::from_raw_parts(return_proper as *mut u8, return_nitems as usize) }
						.iter()
						.map(|x| Window(*x as XWindow, Rc::clone(display)))
						.collect();
					unsafe { XFree(return_proper as *mut c_void) };
					Windows(array)
				}
				16 => {
					let array = unsafe { slice::from_raw_parts(return_proper as *mut u16, return_nitems as usize) }
						.iter()
						.map(|x| Window(*x as XWindow, Rc::clone(display)))
						.collect();
					unsafe { XFree(return_proper as *mut c_void) };
					Windows(array)
				}
				32 => {
					let array = unsafe { slice::from_raw_parts(return_proper as *mut usize, return_nitems as usize) }
						.iter()
						.map(|x| Window(*x as XWindow, Rc::clone(display)))
						.collect();
					unsafe { XFree(return_proper as *mut c_void) };
					Windows(array)
				}
				_ => {
					unsafe { XFree(return_proper as *mut c_void) };
					return Err(NotSupported);
				}
			};
			return Ok(windows);
		} else { unsafe { XFree(return_proper as *mut c_void) }; }

		Err(NotSupported)
	}

	/// Get window by provided name on the screen.
	///
	/// return [Window] on success or [None] if not found or error
	pub fn get_window_by_name(&self, name: impl AsRef<[u8]>) -> Option<Window> {
		let name = name.as_ref();
		let Session { display, .. } = self;
		let root = self.root();
		let atom = self.client_list();

		let GetWindowPropertyResponse {
			actual_type_return: return_type,
			actual_format_return: return_format,
			nitems_return: return_nitems,
			proper_return: return_proper,
			..
		} = unsafe { get_window_property(display, root.clone(), *atom, XA_WINDOW).ok()? };
		if return_type == XA_WINDOW {
			return match return_format {
				8 => {
					let res = unsafe { slice::from_raw_parts(return_proper as *mut u8, return_nitems as usize) }
						.iter()
						.map(|x| Window(*x as XWindow, Rc::clone(display)))
						.find(|it| it.match_title(name));
					unsafe { XFree(return_proper as *mut c_void) };
					res
				}
				16 => {
					let res = unsafe { slice::from_raw_parts(return_proper as *mut u16, return_nitems as usize) }
						.iter()
						.map(|x| Window(*x as XWindow, Rc::clone(display)))
						.find(|it| it.match_title(name));
					unsafe { XFree(return_proper as *mut c_void) };
					res
				}
				32 => {
					let res = unsafe { slice::from_raw_parts(return_proper as *mut usize, return_nitems as usize) }
						.iter()
						.map(|x| Window(*x as XWindow, Rc::clone(display)))
						.find(|it| it.match_title(name));
					unsafe { XFree(return_proper as *mut c_void) };
					res
				}
				_ => {
					unsafe { XFree(return_proper as *mut c_void) };
					None
				}
			};
		} else {
			unsafe { XFree(return_proper as *mut c_void) };
		}

		None
	}
	/// Gets the currently active window in the display.
	pub fn active_window(&mut self) -> Result<Window, NotSupported> {
		Window::active_window(self)
	}
}
