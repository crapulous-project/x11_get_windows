use x11_get_windows::Session;

fn main() {
	let session = Session::open()
		.expect("Error opening a new session.");
	session
		.get_windows()
		.expect("Could not get a list of windows.")
		.iter()
		.filter_map(|x| x.get_title().ok())
		.for_each(|x| println!("{:?}", x.as_ref()));
}
