use std::{io, thread, time, process};
use std::io::Write;		// for flush
use std::fs;
use std::process::Command;
use clap::Parser;

use arboard::Clipboard;

#[cfg(target_os="linux")]
use arboard::{ClipboardExtLinux, LinuxClipboardKind};

#[derive(Parser)]
struct Cli {
    /// Do not print anything to stdout, ignores `separator`
    #[clap(short, long, action)]
    quiet: bool,
    /// Use Primary Selection instead of Clipboard (Linux)
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

    if !atty::is(atty::Stream::Stdout) && args.count == 0 {
	eprintln!("You have to provide the --count > 0 while piped to avoid infinite loop.");
	process::exit(1);
    }

    #[cfg(target_os="linux")]
    let clip;

    #[cfg(target_os="linux")]
    if args.primary {
	clip = Some(LinuxClipboardKind::Primary);
    } else {
	clip = Some(LinuxClipboardKind::Clipboard);
    }


    #[cfg(not(target_os="linux"))]
    if args.primary {
	println!("Primary Clipboard is not available for {}", std::env::consts::OS);
	process::exit(1);
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

    #[cfg(target_os="linux")]
    let mut clip_txt = ctx.get_text_with_clipboard(clip.unwrap()).unwrap_or_else(|_| String::from(""));

    #[cfg(not(target_os="linux"))]
    let mut clip_txt = ctx.get_text().unwrap_or_else(|_| String::from(""));

    let mut counter = 0;
    loop {

	#[cfg(target_os="linux")]
	let clip_new = ctx.get_text_with_clipboard(clip.unwrap()).unwrap_or_else(|_| String::from(""));

	#[cfg(not(target_os="linux"))]
	let clip_new = ctx.get_text().unwrap_or_else(|_| String::from(""));

	if clip_new != clip_txt {

	    if !args.quiet {
		print!("{}{}", clip_new, args.separator);
		io::stdout().flush().unwrap();
	    }
	    if file.is_some(){
		file.as_ref().unwrap().write_all(clip_new.as_bytes()).expect("Unable to write to file.");
		file.as_ref().unwrap().write_all(args.separator.as_bytes()).expect("Unable to write to file.");
	    }
	    if !args.command.is_empty() {
		let mut cmd = Command::new(args.command.clone());
		match cmd.arg(clip_new.clone()).output() {
		    Ok(_) => None,
		    Err(e) => Some(eprintln!("Error Executing Command: {}", e)),
		};
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
