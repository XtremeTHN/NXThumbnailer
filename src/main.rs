use std::io::Read;
use std::io::Seek;
use std::io::Write;

use image::save_buffer_with_format;
use nxroms::formats::nca::ContentType;
use nxroms::formats::nca::Nca;
use nxroms::fs::pfs::{PFSEntry, PFSHeader, PartitionFs};
use nxroms::fs::romfs::RomFs;
use nxroms::fs::romfs::RomFsErrors;
use nxroms::fs::romfs::RomFsFileEntry;
use nxroms::keyring::Keyring;
use nxroms::readers::FileRegion;
use nxroms::{BinRead, ReadAt};

type Error = Box<dyn std::error::Error>;

fn find_control<H, E, S>(pfs: &PartitionFs<H>, entry: &E, stream: &mut S, keyring: &Keyring) -> bool
where
    H: PFSHeader + BinRead,
    E: PFSEntry,
    S: ReadAt,
{
    let name = pfs.get_name_for_entry(entry);

    if let Ok(n) = name.as_ref() && let Some(parts) = n.split_once(".") && parts.1 != "nca" {
        return false;
    }

    let mut raw_entry = pfs.open_entry(entry, stream);
    let nca = Nca::new(keyring, &mut raw_entry);

    match nca {
        Ok(n) => {
            if matches!(n.header.content_type, ContentType::Control) {
                true
            } else {
                false
            }
        }
        Err(e) => {
            eprintln!("WARN: {}: {}", name.unwrap_or(String::from("Unknown")), e);
            false
        }
    }
}

fn find_icon(entry: &Result<RomFsFileEntry, RomFsErrors>) -> bool {
    match entry {
        Ok(e) => {
            if let Ok(name) = e.name()
                && let Some(ext) = name.split_once(".")
                && ext.1 == "dat"
            {
                true
            } else {
                false
            }
        }
        Err(e) => {
            eprintln!("Couldn't parse romfs file entry: {}", e);
            false
        }
    }
}

fn handle_icon<S: ReadAt + Read + Seek>(stream: &mut S) -> Result<(), Error> {
    let buffered = std::io::BufReader::new(stream);
    image::load(buffered, image::ImageFormat::Jpeg)?
        .thumbnail(128, 128)
        .save("out.png")?;

    Ok(())
}

fn parse_pfs<H: PFSHeader + BinRead, S: ReadAt>(pfs: PartitionFs<H>, stream: &mut S) -> Result<(), Error> {
    let mut keyring = Keyring::new("~/.switch/prod.keys");
    keyring.parse()?;

    let Some(control_entry) = pfs
        .header
        .entry_table()
        .iter()
        .find(|e| find_control(&pfs, *e, stream, &keyring))
    else {
        return Err("Couldn't find control nca".into());
    };

    let mut control_raw = pfs.open_entry(control_entry, stream);
    
    let mut control_nca = Nca::new(&keyring, &mut control_raw)?;
    let mut romfs_raw = control_nca.open_fs(0, &mut control_raw)?;

    let romfs = RomFs::new(&mut romfs_raw)?;
    let Some(icon) = romfs.files().find(find_icon) else {
        return Err("No icon available".into());
    };

    let mut icon_raw = romfs.open_file(&icon.unwrap(), &mut romfs_raw);

    handle_icon(&mut icon_raw)
}

fn main() -> Result<(), Error> {
    let mut file = std::fs::File::open("celeste.nsp")?;
    
    let pfs = PartitionFs::new_pfs0(&mut file)?;
    parse_pfs(pfs, &mut file)
}
