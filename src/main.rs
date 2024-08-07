use clap::{ArgGroup, Parser};
use image::{DynamicImage, RgbaImage};
use regex::Regex;
use std::fs;
use std::io::Write;
// for flush
use std::path::Path;
use std::{io, process, thread, time};
use string_template_plus::{Render, RenderOptions, Template};
use subprocess::Exec;

use arboard::{Clipboard, Error, ImageData};

#[cfg(target_os = "linux")]
use arboard::{ClipboardExtLinux, LinuxClipboardKind};

#[derive(Parser)]
#[command(group = ArgGroup::new("text").required(false).multiple(true))]
struct Cli {
    /// Do not print anything to stdout, ignores `separator`
    #[arg(short, long, action)]
    quiet: bool,
    /// Use Primary Selection instead of Clipboard (Linux)
    #[arg(short, long, group = "text", action)]
    primary: bool,
    /// Separator between two entries for output
    #[arg(short, long, group = "text", default_value = "\n")]
    separator: String,
    /// Do not clear output file before writing to it
    #[arg(short, long, group = "text", action)]
    append: bool,
    /// Output File to write the captured contents.
    #[arg(short, long, group = "text")]
    output: Option<std::path::PathBuf>,
    /// Command to run on each entry
    ///
    /// available variables as template
    /// {text} => copied text
    /// {i} => counter
    #[arg(short, long, default_value = "", value_parser=Template::parse_template)]
    command: Template,
    /// Filter the capture to matching regex pattern
    #[arg(short, long, group = "text", default_value = "")]
    filter: String,
    /// Image file template
    #[arg(short, long, conflicts_with = "text", value_parser=Template::parse_template)]
    image: Option<Template>,
    /// Refresh Rate in miliseconds
    #[arg(short, long, default_value = "200")]
    refresh_rate: u64,
    /// Only capture this many times, 0 for infinity
    #[arg(short = 'n', long, default_value = "0")]
    count: u64,
}

fn clipboard_image_changed(
    old: &Result<ImageData<'_>, Error>,
    new: &Result<ImageData<'_>, Error>,
) -> bool {
    match (old, new) {
        (Ok(o), Ok(n)) => {
            if (o.height, o.width) != (n.height, n.width) {
                true
            } else {
                // this is really intensive if it's the same
                // image. and the image is huge.
                o.bytes != o.bytes
            }
        }
        (Err(_), Ok(_)) => true,
        (_, Err(_)) => false,
    }
}

fn save_image<P: AsRef<Path>>(img: ImageData, filename: P) -> bool {
    let img = RgbaImage::from_raw(img.width as u32, img.height as u32, img.bytes.to_vec())
        .expect("Clipboard ImageData is invalid");
    // the saved images are write protected, idk why, need to look into it
    // I don't remember what I mean by above previously, looks fine now
    if let Err(e) = DynamicImage::ImageRgba8(img).save(filename.as_ref()) {
        eprintln!("f:{:?}", e);
        false
    } else {
        true
    }
}

fn clipboard_images(args: Cli) {
    let mut ctx = Clipboard::new().unwrap();
    let mut clip_old = ctx.get_image();
    let mut counter = 0;
    let mut img_count = 1;
    let mut op = RenderOptions::default();
    let template = args.image.unwrap();
    loop {
        let clip_new = ctx.get_image();
        if clipboard_image_changed(&clip_old, &clip_new) {
            match &clip_new {
                Ok(img) => {
                    op.variables.insert("i".into(), img_count.to_string());
                    match template.render(&op) {
                        Ok(templ) => {
                            if save_image(img.clone(), &templ) {
                                println!("Saved: {:?}", templ);
                                img_count += 1;
                            }
                        }
                        Err(e) => eprintln!("{:?}", e),
                    }
                }
                Err(e) => eprintln!("{:?}", e),
            }
            counter += 1;
            if counter == args.count {
                // counter is never 0 here as it starts from 1 as soon
                // as a match is found.
                break;
            }
            clip_old = clip_new;
        }
        thread::sleep(time::Duration::from_millis(args.refresh_rate));
    }
}

fn main() {
    let args = Cli::parse();

    if !atty::is(atty::Stream::Stdout) && args.count == 0 {
        eprintln!("You have to provide the --count > 0 while piped to avoid infinite loop.");
        process::exit(1);
    }

    if args.image.is_some() {
        clipboard_images(args);
        return;
    }

    let regex_pattern = if args.filter.is_empty() {
        None
    } else {
        Some(Regex::new(&args.filter).unwrap())
    };

    #[cfg(target_os = "linux")]
    let clip = if args.primary {
        Some(LinuxClipboardKind::Primary)
    } else {
        Some(LinuxClipboardKind::Clipboard)
    };

    #[cfg(not(target_os = "linux"))]
    if args.primary {
        println!(
            "Primary Clipboard is not available for {}",
            std::env::consts::OS
        );
        process::exit(1);
    }

    let mut output: Box<dyn Write> = match args.output {
        Some(out) => Box::new(
            fs::OpenOptions::new()
                .write(true)
                .create(true)
                .append(args.append)
                .truncate(!args.append)
                .open(out)
                .unwrap(),
        ),
        None => {
            if args.quiet {
                Box::new(std::io::empty())
            } else {
                Box::new(std::io::stdout())
            }
        }
    };
    let mut ctx = Clipboard::new().unwrap();

    #[cfg(target_os = "linux")]
    let mut clip_txt = ctx
        .get_text_with_clipboard(clip.unwrap())
        .unwrap_or_else(|_| String::from(""));

    #[cfg(not(target_os = "linux"))]
    let mut clip_txt = ctx.get_text().unwrap_or_else(|_| String::from(""));

    let mut counter = 0;
    let mut op = RenderOptions::default();
    loop {
        #[cfg(target_os = "linux")]
        let clip_new = ctx
            .get_text_with_clipboard(clip.unwrap())
            .unwrap_or_else(|_| String::from(""));

        #[cfg(not(target_os = "linux"))]
        let clip_new = ctx.get_text().unwrap_or_else(|_| String::from(""));

        if clip_new != clip_txt {
            if regex_pattern.as_ref().is_some()
                && !regex_pattern.as_ref().unwrap().is_match(&clip_new)
            {
                clip_txt = clip_new;
                continue;
            }

            write!(output, "{}{}", clip_new, args.separator).ok();
            io::stdout().flush().unwrap();
            if !args.command.parts().is_empty() {
                op.variables.insert("i".into(), counter.to_string());

                op.variables.insert("text".into(), clip_new.clone());
                match args.command.render(&op) {
                    Ok(cmd) => {
                        match Exec::shell(cmd).join() {
                            Ok(_) => (),
                            Err(e) => eprintln!("Error Executing Command: {}", e),
                        };
                    }
                    Err(e) => eprintln!("{:?}", e),
                }
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
