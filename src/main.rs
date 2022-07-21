use std::{io, thread, time};
use std::io::Write;
use std::process::Command;
use clap::Parser;
use arboard::{Clipboard, ClipboardExtLinux, LinuxClipboardKind};

#[derive(Parser)]
struct Cli {
    /// Use Primary Selection instead of Clipboard
    #[clap(short, long, action)]
    primary: bool,
    /// Character between two entries
    #[clap(short, long, default_value = "\n")]
    separator: String,
    /// Template command to run on each entry
    #[clap(short, long, default_value = "")]
    command: String,
}


fn mainloop(clip:LinuxClipboardKind, args:Cli) {
    let mut ctx = Clipboard::new().unwrap();
    let mut clip_txt = ctx.get_text_with_clipboard(clip).unwrap();
    loop {
	let clip_new = ctx.get_text_with_clipboard(clip).unwrap();

	if clip_new != clip_txt {
	    print!("{}{}", clip_new, args.separator);
	    io::stdout().flush().unwrap();
	    if !args.command.is_empty() {
		let mut cmd = Command::new(args.command.clone());
		cmd.arg(clip_new.clone()).output().expect("Command Failed.");
	    }
	    clip_txt = clip_new;
	}
	thread::sleep(time::Duration::from_millis(10));
    }
}

fn main() {
    let args = Cli::parse();
    let mut clipboard = LinuxClipboardKind::Clipboard;
    if args.primary {
	clipboard = LinuxClipboardKind::Primary;
    }
    mainloop(clipboard, args);
}
