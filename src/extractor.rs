use std::io::Read;
use std::io::Seek;

use image::DynamicImage;
use nxroms::formats::nca::ContentType;
use nxroms::formats::nca::Nca;
use nxroms::fs::pfs::{PFSEntry, PFSHeader, PartitionFs};
use nxroms::fs::romfs::RomFs;
use nxroms::fs::romfs::RomFsErrors;
use nxroms::fs::romfs::RomFsFileEntry;
use nxroms::keyring::Keyring;
use nxroms::{BinRead, ReadAt};

type Error = Box<dyn std::error::Error>;

fn find_control<H, E, S>(pfs: &PartitionFs<H>, entry: &E, stream: &mut S, keyring: &Keyring) -> bool
where
    H: PFSHeader + BinRead,
    E: PFSEntry,
    S: ReadAt,
{
    let name = pfs.get_name_for_entry(entry);

    if let Ok(n) = name.as_ref()
        && let Some(parts) = n.split_once(".")
        && parts.1 != "nca"
    {
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
            log::warn!(
                "Failed to parse nca \"{}\": {}",
                name.unwrap_or(String::from("Unknown")),
                e
            );
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
            log::warn!("Failed to parse romfs file entry: {}", e);
            false
        }
    }
}

pub fn extract_icon<H, S>(pfs: PartitionFs<H>, stream: &mut S) -> Result<DynamicImage, Error>
where
    H: PFSHeader + BinRead,
    S: ReadAt + Read + Seek,
{
    log::info!("Reading keyring...");
    let mut keyring = Keyring::new("~/.switch/prod.keys");
    keyring.parse()?;

    log::debug!("Searching control nca...");
    let Some(control_entry) = pfs
        .header
        .entry_table()
        .iter()
        .find(|e| find_control(&pfs, *e, stream, &keyring))
    else {
        return Err("Couldn't find control nca".into());
    };
    log::debug!("Found: \"{}\"", pfs.get_name_for_entry(control_entry)?);

    let mut control_raw = pfs.open_entry(control_entry, stream);

    let mut control_nca = Nca::new(&keyring, &mut control_raw)?;
    let mut romfs_raw = control_nca.open_fs(0, &mut control_raw)?;

    let romfs = RomFs::new(&mut romfs_raw)?;

    log::debug!("Searching rom icon...");
    let Some(icon) = romfs.files().find(find_icon) else {
        return Err("No icon available".into());
    };
    log::debug!(
        "Found: {:?}",
        icon.as_ref()
            .unwrap()
            .name()
            .unwrap_or(String::from("Unknown"))
    );

    log::info!("Extracting thumbnail...");
    let mut icon_raw = romfs.open_file(&icon.unwrap(), &mut romfs_raw);
    let buffered = std::io::BufReader::new(&mut icon_raw);
    Ok(image::load(buffered, image::ImageFormat::Jpeg)?)
}
