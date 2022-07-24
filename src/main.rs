use std::{io, thread, time};
use std::io::Write;		// for flush
use std::fs;
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
    /// Separator between two entries for output
    #[clap(short, long, default_value = "\n")]
    separator: String,
    /// Do not clear output file before writing to it
    #[clap(short, long, action)]
    append: bool,
    /// Output File to write the captured contents.
    #[clap(parse(from_os_str), short, long, default_value = "")]
    output: std::path::PathBuf,
    /// Command to run on each entry
    #[clap(short, long, default_value = "")]
    command: String,
    /// Refresh Rate in miliseconds
    #[clap(short, long, default_value = "200")]
    refresh_rate: u64,
    /// Only capture this many times, 0 for infinity
    #[clap(short='n', long, default_value = "0")]
    count: u64,
}


fn main() {
    let args = Cli::parse();

    let mut clip = LinuxClipboardKind::Clipboard;
    // TODO mark this unavailable for windows
    if args.primary {
	clip = LinuxClipboardKind::Primary;
    }
    
    let mut file:Option<fs::File> = None;
    if !args.output.as_os_str().is_empty() {
	file = Some(fs::OpenOptions::new()
		    .write(true)
		    .create(true)
		    .append(args.append)
		    .truncate(!args.append)
		    .open(args.output)
		    .unwrap());
    }
    let mut ctx = Clipboard::new().unwrap();
    let mut clip_txt = ctx.get_text_with_clipboard(clip).unwrap_or_else(|_| String::from(""));

    let mut counter = 0;
    loop {
	let clip_new = ctx.get_text_with_clipboard(clip).unwrap_or_else(|_| String::from(""));

	if clip_new != clip_txt {

	    if !args.quiet {
		print!("{}{}", clip_new, args.separator);
		io::stdout().flush().unwrap();
	    }
	    if !file.is_none(){
		file.as_ref().unwrap().write_all(clip_new.as_bytes()).expect("Unable to write to file.");
		file.as_ref().unwrap().write_all(args.separator.as_bytes()).expect("Unable to write to file.");
	    }
	    if !args.command.is_empty() {
		let mut cmd = Command::new(args.command.clone());
		cmd.arg(clip_new.clone()).output().expect("Command Failed.");
	    }

	    clip_txt = clip_new;
	    counter += 1;
	    if counter == args.count {
		// counter is never 0 here as it starts from 1 as soon
		// as a match is found.
		break;
	    }
	}
	thread::sleep(time::Duration::from_millis(args.refresh_rate));
    }
}
