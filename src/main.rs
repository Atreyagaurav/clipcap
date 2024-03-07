use clap::{ArgGroup, Parser};
use image::{DynamicImage, RgbaImage};
use regex::Regex;
use std::fs;
use std::io::Write;
// for flush
use std::path::Path;
use std::process::Command;
use std::{io, process, thread, time};
use string_template_plus::{Render, RenderOptions, Template};

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
    #[arg(short, long, default_value = "")]
    command: String,
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

fn save_image<P: AsRef<Path>>(img: &ImageData, filename: P) -> bool {
    let img = RgbaImage::from_raw(img.width as u32, img.height as u32, img.bytes.to_vec())
        .expect("Clipboard ImageData is invalid");
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
                            if save_image(img, &templ) {
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

    let mut file: Option<fs::File> = None;
    if args.output.is_some() {
        file = Some(
            fs::OpenOptions::new()
                .write(true)
                .create(true)
                .append(args.append)
                .truncate(!args.append)
                .open(args.output.unwrap())
                .unwrap(),
        );
    }
    let mut ctx = Clipboard::new().unwrap();

    #[cfg(target_os = "linux")]
    let mut clip_txt = ctx
        .get_text_with_clipboard(clip.unwrap())
        .unwrap_or_else(|_| String::from(""));

    #[cfg(not(target_os = "linux"))]
    let mut clip_txt = ctx.get_text().unwrap_or_else(|_| String::from(""));

    let mut counter = 0;
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

            if !args.quiet {
                print!("{}{}", clip_new, args.separator);
                io::stdout().flush().unwrap();
            }
            if file.is_some() {
                file.as_ref()
                    .unwrap()
                    .write_all(clip_new.as_bytes())
                    .expect("Unable to write to file.");
                file.as_ref()
                    .unwrap()
                    .write_all(args.separator.as_bytes())
                    .expect("Unable to write to file.");
            }
            if !args.command.is_empty() {
                let mut cmd = Command::new(args.command.clone());
                match cmd.arg(clip_new.clone()).output() {
                    Ok(_) => (),
                    Err(e) => eprintln!("Error Executing Command: {}", e),
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
