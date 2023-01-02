use std::mem;
use std::sync::RwLock;

pub trait RwLockCell<T> {
	fn get_or_insert_with<F: FnOnce() -> T>(&self, f: F) -> &T;
}

impl<T> RwLockCell<T> for RwLock<Option<T>> {
	fn get_or_insert_with<F: FnOnce() -> T>(&self, f: F) -> &T {
		let read = self.read().unwrap();
		if let Some(val) = read.as_ref() {
			unsafe { mem::transmute(val) }
		} else {
			drop(read);
			let mut write = self.write().unwrap();
			*write = Some(f());
			unsafe { mem::transmute(write.as_ref().unwrap()) }
		}
	}
}