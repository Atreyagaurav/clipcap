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
    /// Character between two entries
    #[clap(short, long, default_value = "\n")]
    separator: String,
    /// Do not clear output file before writing to it
    #[clap(short, long, action)]
    append: bool,
    /// Output File to write the captured contents. By default it'll
    /// make a file in /tmp and write the output there
    #[clap(parse(from_os_str), short, long, default_value = "")]
    // TODO
    // couldn't figure out how to use `env::temp_dir().join("clipcap.txt").to_str().unwrap()`
    output: std::path::PathBuf,
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
    // TODO mark this unavailable for windows
    if args.primary {
	clip = LinuxClipboardKind::Primary;
    }

    let mut write_to_file = false;

    // TODO need a way to not open file without need, conditional
    // fails to compile. this file isn't actually used but compiler
    // has problems with me not having file initialized.
    let mut file = fs::OpenOptions::new().read(true).open("/dev/null").unwrap();
    
    
    if !args.output.as_os_str().is_empty(){
	write_to_file = true;
    }
    
    if write_to_file {
	file = fs::OpenOptions::new()
	    .write(true)
	    .create(true)
	    .append(args.append)
	    .truncate(!args.append)
	    .open(&args.output)
	    .unwrap();	
    }
    
    let mut ctx = Clipboard::new().unwrap();
    let mut clip_txt = ctx.get_text_with_clipboard(clip).unwrap_or_else(|_| String::from(""));
    loop {
	let clip_new = ctx.get_text_with_clipboard(clip).unwrap_or_else(|_| String::from(""));

	if clip_new != clip_txt {
	    if !args.quiet {
		print!("{}{}", clip_new, args.separator);
		io::stdout().flush().unwrap();
	    }

	    if write_to_file {
		file.write_all(clip_new.as_bytes()).expect("Unable to write to file.");
		file.write_all(args.separator.as_bytes()).expect("Unable to write to file.");
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
