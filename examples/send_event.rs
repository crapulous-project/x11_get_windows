use x11::keysym::XK_F1;

use x11_get_windows::event::key_event::KeyType;
use x11_get_windows::Session;

fn main() {
	let session = Session::open()
		.expect("Error opening a new session.");
	let winname = b"Chrome";
	let win = session
		.find_window(|it| it.windows(winname.len()).any(|it| it == winname), 0)
		.into_iter()
		.next()
		.expect("You don't have chrome window open");
	println!("Chrome title: {:?}", win.get_title());

	// Send F1 key to chrome
	
	// focus before send key
	win.focus(); 
	// press key
	win.send_key(KeyType::Press, XK_F1, 0);
	// release key
	win.send_key(KeyType::Release, XK_F1, 0);
}
