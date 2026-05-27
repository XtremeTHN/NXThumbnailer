mod extractor;

use std::fs::File;
use std::path::PathBuf;
use std::process::exit;
use image::DynamicImage;
use clap::Parser;

use crate::extractor::extract_icon;
use nxroms::{formats::xci::{Xci, XciPartition}, fs::{hfs::HashPartitionFsHeader, pfs::{PartitionFs, PartitionFsHeader}}};

type AnyError<R> = Result<R, Box<dyn std::error::Error>>;

#[derive(Parser)]
struct Args {
    #[arg(short = 'i')]
    pub input: PathBuf,

    #[arg(short = 'o')]
    pub output: PathBuf,

    #[arg(short = 'k')]
    pub keys_path: Option<PathBuf>,

    #[arg(short = 's')]
    pub size: u32,

    #[arg(short = 'v')]
    pub verbose: bool,

    #[arg(short = 'q')]
    pub quiet: bool
}

enum RomType {
    Nsp,
    Xci
}

fn validate_input(input: &PathBuf) -> RomType {
    if !input.exists() {
        log::error!("Input file does not exist: {:?}", input);
        exit(1);
    }

    let f = input.extension().and_then(|e| {
        if e == "nsp" {
            Some(RomType::Nsp)
        } else if e == "xci" {
            Some(RomType::Xci)
        } else {
            None
        }
    });

    if f.is_none() {
        log::error!("Input file has an invalid extension: {:?}", input);
        exit(2);
    }

    f.unwrap()
}

fn extract_pfs_nsp(file: &mut File) -> AnyError<PartitionFs<PartitionFsHeader>> {
    Ok(PartitionFs::new_pfs0(file)?)
}

fn extract_pfs_xci(file: &mut File) -> AnyError<PartitionFs<HashPartitionFsHeader>> {
    let mut xci = Xci::new(file)?;

    let mut raw_pfs = xci.open_partition(XciPartition::Secure, file)?;
    Ok(xci.open_partition_fs(&mut raw_pfs)?)
}

fn process(args: &Args, rom_type: RomType) -> AnyError<()> {
    let mut file = File::open(&args.input)?;
    let img = match rom_type {
        RomType::Nsp => {
            extract_icon(extract_pfs_nsp(&mut file)?, &mut file)?
        }
        RomType::Xci => {
            extract_icon(extract_pfs_xci(&mut file)?, &mut file)?
        }
    };

    log::info!("Saving thumbnail...");
    img
        .thumbnail(args.size, args.size)
        .save(&args.output)?;

    Ok(())
}

fn main() {
    let args = Args::parse();

    let level = if args.verbose {
        "debug"
    } else {
        "info"
    };

    let env = env_logger::Env::new().filter_or("LIFT_LOG", level);

    if !args.quiet {
        env_logger::init_from_env(env);
    }

    let _type = validate_input(&args.input);

    if let Err(e) = process(&args, _type) {
        log::error!("Failed to create rom thumbnail for file {:?}: {}", args.input, e);
        std::process::exit(2);
    };
}