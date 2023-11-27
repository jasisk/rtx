use std::fs::File;
use std::path::Path;

use eyre::Result;
use flate2::read::GzDecoder;

use crate::file;

pub fn untar(tarball: &Path, dest: &Path) -> Result<()> {
    debug!(
        "untar {} to {}",
        file::file_name(tarball).unwrap(),
        dest.display()
    );
    let tar = GzDecoder::new(File::open(tarball)?);
    let mut archive = tar::Archive::new(tar);
    archive.unpack(dest)?;
    Ok(())
}
