use std::{io, thread, time};
use std::io::Write;		// for flush
use std::process::Command;
use clap::Parser;
use arboard::{Clipboard, ClipboardExtLinux, LinuxClipboardKind};

#[derive(Parser)]
struct Cli {
    /// Do not print anything to stdout, ignores `separator`
    #[clap(short, long, action)]
    quiet: bool,
    /// Use Primary Selection instead of Clipboard
    #[clap(short, long, action)]
    primary: bool,
    /// Character between two entries
    #[clap(short, long, default_value = "\n")]
    separator: String,
    /// Command to run on each entry
    #[clap(short, long, default_value = "")]
    command: String,
    /// Refresh Rate in miliseconds
    #[clap(short, long, default_value = "200")]
    refresh_rate: u64,
}


fn main() {
    let args = Cli::parse();

    let mut clip = LinuxClipboardKind::Clipboard;
    if args.primary {
	clip = LinuxClipboardKind::Primary;
    }
    
    let mut ctx = Clipboard::new().unwrap();
    let mut clip_txt = ctx.get_text_with_clipboard(clip).unwrap();
    loop {
	let clip_new = ctx.get_text_with_clipboard(clip).unwrap();

	if clip_new != clip_txt {
	    if !args.quiet {
		print!("{}{}", clip_new, args.separator);
		io::stdout().flush().unwrap();
	    }
	    if !args.command.is_empty() {
		let mut cmd = Command::new(args.command.clone());
		cmd.arg(clip_new.clone()).output().expect("Command Failed.");
	    }
	    clip_txt = clip_new;
	}
	thread::sleep(time::Duration::from_millis(args.refresh_rate));
    }
}
