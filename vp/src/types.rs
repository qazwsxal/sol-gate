use std::{fmt::Display, slice::Iter};

// Header and Index for use with nom parser.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VPHeader {
    pub version: u32,
    pub offset: u32,
    pub entries: u32,
}

#[derive(Clone, Debug)]
pub struct VPIndex {
    pub fileoffset: u32,
    pub size: u32,
    pub name: [u8; 32],
    pub timestamp: u32,
}

#[derive(Debug)]
pub enum VPError {
    NotFound,
}

impl Display for VPError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for VPError {}

// Entry, Dir and File for actually parsing into a directory structure.
#[derive(Debug, Clone)]
pub enum VPEntry {
    File(VPFile),
    Dir(VPDir),
}

#[derive(Clone, Default, Debug)]
pub struct VPDir {
    pub name: String,
    pub contents: Vec<VPEntry>,
}

impl VPDir {
    pub fn locate(&self, filepath: &[String]) -> Result<VPFile, VPError> {
        let folder = &filepath[0];
        let subentry = self.contents.iter().find(|f| match f {
            VPEntry::Dir(x) => &x.name == folder,
            VPEntry::File(x) => &x.name == folder,
        });

        match subentry {
            None => Err(VPError::NotFound),
            Some(VPEntry::Dir(dir)) => dir.locate(&filepath[1..]),
            Some(VPEntry::File(file)) => Ok(file.clone()),
        }
    }

    pub fn flatten(&self) -> Vec<VPFile> {
        self.contents
            .iter()
            .flat_map(|e| match e {
                VPEntry::Dir(d) => d.flatten(),
                VPEntry::File(f) => vec![f.clone()],
            })
            .map(|f| VPFile {
                fileoffset: f.fileoffset,
                size: f.size,
                name: [self.name.clone(), f.name.clone()].join("/"),
                timestamp: f.timestamp,
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct VPFile {
    pub fileoffset: u64,
    pub size: u64,
    pub name: String,
    pub timestamp: u32,
}

impl From<VPIndex> for VPFile {
    fn from(vpi: VPIndex) -> Self {
        Self {
            fileoffset: vpi.fileoffset.into(),
            size: vpi.size.into(),
            name: std::str::from_utf8(&vpi.name)
                .unwrap()
                .trim_matches(char::from(0))
                .to_string(),
            timestamp: vpi.timestamp,
        }
    }
}

impl From<&mut Iter<'_, VPIndex>> for VPDir {
    fn from(vvpi: &mut Iter<VPIndex>) -> Self {
        let mut vpdir = VPDir::default();
        while let Some(vpi) = vvpi.next() {
            let vpi_name = std::str::from_utf8(&vpi.name)
                .unwrap()
                .trim_matches(char::from(0));
            match vpi.size {
                // if size is 0, we're defining a directory
                0 => {
                    match vpi_name {
                        ".." => break, // End of folder, so return vpdir with full contents vector
                        &_ => vpdir.contents.push(VPEntry::Dir({
                            let mut v = VPDir::from(&mut *vvpi);
                            v.name = vpi_name.to_string();
                            v
                        })),
                    }
                } //i+1 here as we're iterating from [1..]
                _ => vpdir
                    .contents
                    .push(VPEntry::File(VPFile::from(vpi.clone()))),
            }
        }
        vpdir
    }
}

impl From<Vec<VPIndex>> for VPDir {
    fn from(vvpi: Vec<VPIndex>) -> Self {
        Self::from(vvpi.iter())
    }
}

impl From<Iter<'_, VPIndex>> for VPDir {
    fn from(mut vvpi: Iter<'_, VPIndex>) -> Self {
        Self::from(&mut vvpi)
    }
}
