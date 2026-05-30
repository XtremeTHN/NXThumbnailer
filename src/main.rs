mod extractor;

use std::fs::File;
use std::path::PathBuf;
use std::process::exit;
use clap::Parser;
use gio::prelude::FileExt;

use crate::extractor::extract_icon;
use nxroms::{formats::xci::{Xci, XciPartition}, fs::{hfs::HashPartitionFsHeader, pfs::{PartitionFs, PartitionFsHeader}}};

type AnyError<R> = Result<R, Box<dyn std::error::Error>>;

#[derive(Parser)]
struct Args {
    #[arg(short = 'i')]
    pub input: String,

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

fn extract_pathbuf(file: &gio::File) -> PathBuf {
    let p = file.path();
    if p.is_none() {
        log::error!("Couldn't get a path to file: {}", file.uri());
        exit(10);
    }

    p.unwrap()
}

fn get_rom_type(input: &gio::File) -> RomType {
    if !input.query_exists(None::<&gio::Cancellable>) {
        log::error!("Input file does not exist: {:?}", input);
        exit(1);
    }


    let path = extract_pathbuf(input);
    let f = path.extension().and_then(|e| {
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
        exit(3);
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

fn process(args: &Args, file: gio::File, rom_type: RomType) -> AnyError<()> {
    let input = extract_pathbuf(&file);
    println!("{:?}", input);
    let mut file = File::open(input)?;
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

    let file = gio::File::for_uri(&args.input);

    let _type = get_rom_type(&file);

    if let Err(e) = process(&args, file, _type) {
        log::error!("Failed to create rom thumbnail for file {:?}: {}", args.input, e);
        std::process::exit(2);
    };
}
