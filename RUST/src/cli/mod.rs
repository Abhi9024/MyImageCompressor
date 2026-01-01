//! Command-line interface for the medical image compression tool.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use crate::config::{CompressionCodec, CompressionConfig, CompressionMode, QualityPreset};
use crate::dicom::DicomFile;
use crate::error::Result;
use crate::pipeline::{CompressionPipeline, CompressionResult};

/// Medical Image Compression Tool
///
/// A high-performance DICOM image compression utility supporting
/// JPEG 2000 and JPEG-LS codecs with full regulatory compliance.
#[derive(Parser, Debug)]
#[command(name = "medimg")]
#[command(author = "Medical Imaging Team")]
#[command(version = "0.1.0")]
#[command(about = "Medical image compression supporting JPEG 2000 and JPEG-LS")]
#[command(long_about = None)]
pub struct Cli {
    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress all output except errors
    #[arg(short, long, global = true)]
    pub quiet: bool,
}

/// CLI subcommands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Compress a DICOM file
    Compress {
        /// Input DICOM file path
        #[arg(short, long)]
        input: PathBuf,

        /// Output file path (optional for analysis mode)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Compression codec to use
        #[arg(short, long, value_enum, default_value = "jpeg2000")]
        codec: CodecArg,

        /// Compression mode
        #[arg(short, long, value_enum, default_value = "lossless")]
        mode: ModeArg,

        /// Quality preset (for lossy compression)
        #[arg(short = 'Q', long, value_enum, default_value = "diagnostic")]
        quality: QualityArg,

        /// Target compression ratio (for lossy mode)
        #[arg(short = 'r', long)]
        ratio: Option<f32>,

        /// Near-lossless error tolerance (JPEG-LS only, 0-255)
        #[arg(long, default_value = "0")]
        near: u8,

        /// Verify lossless compression by round-trip decode
        #[arg(long)]
        verify: bool,

        /// Override modality safety checks (use with caution)
        #[arg(long)]
        force: bool,

        /// Dry run - analyze without writing output
        #[arg(long)]
        dry_run: bool,
    },

    /// Show information about a DICOM file
    Info {
        /// Input DICOM file path
        #[arg(short, long)]
        input: PathBuf,

        /// Show detailed metadata
        #[arg(long)]
        detailed: bool,
    },

    /// Analyze compression potential without modifying files
    Analyze {
        /// Input DICOM file path
        #[arg(short, long)]
        input: PathBuf,

        /// Codec to analyze
        #[arg(short, long, value_enum, default_value = "jpeg2000")]
        codec: CodecArg,

        /// Test both lossless and lossy modes
        #[arg(long)]
        all_modes: bool,
    },
}

/// Compression codec argument.
#[derive(ValueEnum, Clone, Debug)]
pub enum CodecArg {
    /// JPEG 2000 (recommended for most use cases)
    Jpeg2000,
    /// JPEG-LS (faster, good for simple images)
    JpegLs,
}

impl From<CodecArg> for CompressionCodec {
    fn from(arg: CodecArg) -> Self {
        match arg {
            CodecArg::Jpeg2000 => CompressionCodec::Jpeg2000,
            CodecArg::JpegLs => CompressionCodec::JpegLs,
        }
    }
}

/// Compression mode argument.
#[derive(ValueEnum, Clone, Debug)]
pub enum ModeArg {
    /// Lossless compression (exact reconstruction)
    Lossless,
    /// Lossy compression (higher ratio, some quality loss)
    Lossy,
    /// Near-lossless (JPEG-LS only)
    NearLossless,
}

impl From<ModeArg> for CompressionMode {
    fn from(arg: ModeArg) -> Self {
        match arg {
            ModeArg::Lossless => CompressionMode::Lossless,
            ModeArg::Lossy => CompressionMode::Lossy,
            ModeArg::NearLossless => CompressionMode::NearLossless,
        }
    }
}

/// Quality preset argument.
#[derive(ValueEnum, Clone, Debug)]
pub enum QualityArg {
    /// Diagnostic quality (lossless)
    Diagnostic,
    /// High quality lossy
    HighQuality,
    /// Standard quality
    Standard,
    /// Preview quality
    Preview,
}

impl From<QualityArg> for QualityPreset {
    fn from(arg: QualityArg) -> Self {
        match arg {
            QualityArg::Diagnostic => QualityPreset::Diagnostic,
            QualityArg::HighQuality => QualityPreset::HighQuality,
            QualityArg::Standard => QualityPreset::Standard,
            QualityArg::Preview => QualityPreset::Preview,
        }
    }
}

/// Run the CLI application.
pub fn run(cli: Cli) -> Result<()> {
    // Initialize logging
    if cli.verbose {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
            .init();
    } else if !cli.quiet {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
            .init();
    }

    match cli.command {
        Commands::Compress {
            input,
            output,
            codec,
            mode,
            quality,
            ratio,
            near,
            verify,
            force,
            dry_run,
        } => {
            run_compress(
                input,
                output,
                codec.into(),
                mode.into(),
                quality.into(),
                ratio,
                near,
                verify,
                force,
                dry_run,
                cli.quiet,
            )
        }
        Commands::Info { input, detailed } => run_info(input, detailed, cli.quiet),
        Commands::Analyze {
            input,
            codec,
            all_modes,
        } => run_analyze(input, codec.into(), all_modes, cli.quiet),
    }
}

/// Run compression command.
fn run_compress(
    input: PathBuf,
    _output: Option<PathBuf>,
    codec: CompressionCodec,
    mode: CompressionMode,
    quality: QualityPreset,
    ratio: Option<f32>,
    near: u8,
    verify: bool,
    force: bool,
    dry_run: bool,
    quiet: bool,
) -> Result<()> {
    let config = CompressionConfig {
        codec,
        mode,
        quality,
        target_ratio: ratio.or_else(|| quality.target_ratio()),
        quality_layers: quality.quality_layers(),
        near_lossless_error: near,
        verify_compression: verify,
        override_safety_checks: force,
        ..Default::default()
    };

    let pipeline = CompressionPipeline::new(config).dry_run(dry_run);
    let result = pipeline.compress_file(&input)?;

    if !quiet {
        print_compression_result(&result);
    }

    Ok(())
}

/// Run info command.
fn run_info(input: PathBuf, detailed: bool, quiet: bool) -> Result<()> {
    let dicom = DicomFile::open(&input)?;
    let metadata = &dicom.metadata;

    if quiet {
        return Ok(());
    }

    println!("DICOM File Information");
    println!("======================");
    println!("File: {}", input.display());
    println!();

    println!("Image Properties:");
    println!("  Dimensions: {}x{}", metadata.width, metadata.height);
    println!("  Bits Stored: {}", metadata.bits_stored);
    println!("  Bits Allocated: {}", metadata.bits_allocated);
    println!("  Samples/Pixel: {}", metadata.samples_per_pixel);
    println!(
        "  Photometric: {}",
        metadata.photometric_interpretation
    );
    println!("  Frames: {}", metadata.number_of_frames);
    println!(
        "  Signed: {}",
        if metadata.pixel_representation == 1 {
            "Yes"
        } else {
            "No"
        }
    );
    println!();

    println!("Transfer Syntax:");
    println!("  UID: {}", metadata.transfer_syntax);
    println!(
        "  Name: {}",
        crate::dicom::utils::transfer_syntax_name(&metadata.transfer_syntax)
    );
    println!(
        "  Compressed: {}",
        if dicom.is_compressed() { "Yes" } else { "No" }
    );
    println!();

    println!("Modality: {:?}", metadata.modality);
    if metadata.modality.requires_lossless() {
        println!("  Note: This modality requires lossless compression (FDA/ACR)");
    }

    if detailed {
        println!();
        println!("DICOM UIDs:");
        if let Some(ref uid) = metadata.patient_id {
            println!("  Patient ID: {}", uid);
        }
        if let Some(ref uid) = metadata.study_uid {
            println!("  Study UID: {}", uid);
        }
        if let Some(ref uid) = metadata.series_uid {
            println!("  Series UID: {}", uid);
        }
        if let Some(ref uid) = metadata.sop_instance_uid {
            println!("  SOP Instance UID: {}", uid);
        }
    }

    // Calculate pixel data size
    let expected_size = crate::dicom::utils::calculate_pixel_data_size(metadata);
    println!();
    println!("Pixel Data:");
    println!("  Expected Size: {} bytes ({:.2} MB)", expected_size, expected_size as f64 / 1_048_576.0);

    Ok(())
}

/// Run analyze command.
fn run_analyze(
    input: PathBuf,
    codec: CompressionCodec,
    all_modes: bool,
    quiet: bool,
) -> Result<()> {
    if all_modes {
        // Test both lossless and lossy
        let lossless_config = CompressionConfig::lossless(codec);
        let lossy_config = CompressionConfig::lossy(codec, 10.0);

        let pipeline_lossless = CompressionPipeline::new(lossless_config);
        let pipeline_lossy = CompressionPipeline::new(lossy_config);

        if !quiet {
            println!("Compression Analysis: {}", input.display());
            println!("========================================");
            println!();
        }

        println!("Lossless Mode:");
        match pipeline_lossless.analyze(&input) {
            Ok(result) => print_compression_result(&result),
            Err(e) => println!("  Error: {}", e),
        }

        println!();
        println!("Lossy Mode (10:1 target):");
        match pipeline_lossy.analyze(&input) {
            Ok(result) => print_compression_result(&result),
            Err(e) => println!("  Error: {}", e),
        }
    } else {
        let config = CompressionConfig::lossless(codec);
        let pipeline = CompressionPipeline::new(config);
        let result = pipeline.analyze(&input)?;

        if !quiet {
            println!("Compression Analysis: {}", input.display());
            println!("========================================");
            println!();
        }

        print_compression_result(&result);
    }

    Ok(())
}

/// Print compression result.
fn print_compression_result(result: &CompressionResult) {
    println!("Compression Result:");
    println!("  Codec: {}", result.codec_name);
    println!(
        "  Mode: {}",
        if result.is_lossless {
            "Lossless"
        } else {
            "Lossy"
        }
    );
    println!(
        "  Original Size: {} bytes ({:.2} MB)",
        result.original_size,
        result.original_size as f64 / 1_048_576.0
    );
    println!(
        "  Compressed Size: {} bytes ({:.2} MB)",
        result.compressed_size,
        result.compressed_size as f64 / 1_048_576.0
    );
    println!("  Compression Ratio: {:.2}:1", result.compression_ratio);
    println!(
        "  Space Savings: {:.1}%",
        result.space_savings_percent()
    );
    println!("  Time: {} ms", result.compression_time_ms);

    if !result.warnings.is_empty() {
        println!();
        println!("Warnings:");
        for warning in &result.warnings {
            println!("  - {}", warning);
        }
    }
}
