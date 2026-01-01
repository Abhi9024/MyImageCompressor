//! Medical Image Compression CLI Tool
//!
//! A command-line utility for compressing DICOM medical images using
//! JPEG 2000 and JPEG-LS codecs.

use clap::Parser;
use medimg_compress::cli::{run, Cli};
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::FAILURE
        }
    }
}
