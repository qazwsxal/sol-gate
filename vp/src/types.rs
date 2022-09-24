use std::slice::Iter;

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

// Entry, Dir and File for actually parsing into a directory structure.
#[derive(Clone, Debug)]
pub enum VPEntry {
    File(VPFile),
    Dir(VPDir),
}

#[derive(Clone, Default, Debug)]
pub struct VPDir {
    pub name: String,
    pub contents: Vec<VPEntry>,
}

#[derive(Clone, Debug)]
pub struct VPFile {
    pub fileoffset: u32,
    pub size: u32,
    pub name: String,
    pub timestamp: u32,
}

impl From<VPIndex> for VPFile {
    fn from(vpi: VPIndex) -> Self {
        Self {
            fileoffset: vpi.fileoffset,
            size: vpi.size,
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
