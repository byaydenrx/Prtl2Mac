//! Just enough of the zip format to unpack a downloaded mod: stored and
//! deflated entries, CRC-checked, written under one folder only (entries
//! that would land outside it are refused). No zip64, no encryption.
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use crate::Error;

const EOCD: u32 = 0x0605_4b50;
const CENTRAL: u32 = 0x0201_4b50;
const LOCAL: u32 = 0x0403_4b50;
/// Refuse archives that claim to unpack to more than this (zip bombs).
const MAX_TOTAL: u64 = 16 << 30;

fn bad(msg: impl Into<String>) -> Error {
    Error::Archive(msg.into())
}

fn u16_at(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}

fn u32_at(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

/// One file (or folder) in the archive.
#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    method: u16,
    crc: u32,
    compressed: u64,
    pub size: u64,
    local_offset: u64,
}

impl Entry {
    pub fn is_dir(&self) -> bool {
        self.name.ends_with('/')
    }
}

/// The entries listed in the central directory.
pub fn entries(file: &mut File) -> Result<Vec<Entry>, Error> {
    let len = file.seek(SeekFrom::End(0))?;
    // The end record is 22 bytes plus a comment of up to 64 KiB.
    let tail_len = len.min(22 + 65535);
    file.seek(SeekFrom::Start(len - tail_len))?;
    let mut tail = vec![0; tail_len as usize];
    file.read_exact(&mut tail)?;
    let eocd = (0..tail.len().saturating_sub(21)).rev().find(|&i| u32_at(&tail, i) == EOCD).ok_or_else(|| bad("not a zip file"))?;
    let count = u16_at(&tail, eocd + 10) as usize;
    let cd_size = u32_at(&tail, eocd + 12) as u64;
    let cd_offset = u32_at(&tail, eocd + 16) as u64;
    if cd_offset == 0xFFFF_FFFF || count == 0xFFFF {
        return Err(bad("zip64 archives are not supported"));
    }
    if cd_offset + cd_size > len {
        return Err(bad("truncated zip file"));
    }
    file.seek(SeekFrom::Start(cd_offset))?;
    let mut cd = vec![0; cd_size as usize];
    file.read_exact(&mut cd)?;
    let mut out = Vec::with_capacity(count);
    let mut i = 0;
    for _ in 0..count {
        if i + 46 > cd.len() || u32_at(&cd, i) != CENTRAL {
            return Err(bad("damaged central directory"));
        }
        let flags = u16_at(&cd, i + 8);
        let method = u16_at(&cd, i + 10);
        let crc = u32_at(&cd, i + 16);
        let compressed = u32_at(&cd, i + 20) as u64;
        let size = u32_at(&cd, i + 24) as u64;
        let name_len = u16_at(&cd, i + 28) as usize;
        let extra_len = u16_at(&cd, i + 30) as usize;
        let comment_len = u16_at(&cd, i + 32) as usize;
        let local_offset = u32_at(&cd, i + 42) as u64;
        let name_end = i + 46 + name_len;
        if name_end > cd.len() {
            return Err(bad("damaged central directory"));
        }
        let name = String::from_utf8_lossy(&cd[i + 46..name_end]).replace('\\', "/");
        if flags & 1 != 0 {
            return Err(bad(format!("{name} is encrypted")));
        }
        if compressed == 0xFFFF_FFFF || size == 0xFFFF_FFFF || local_offset == 0xFFFF_FFFF {
            return Err(bad("zip64 archives are not supported"));
        }
        out.push(Entry { name, method, crc, compressed, size, local_offset });
        i = name_end + extra_len + comment_len;
    }
    Ok(out)
}

/// Unpack every entry of `zip` under `dest` (which must exist).
pub fn extract(zip: &Path, dest: &Path) -> Result<usize, Error> {
    let mut file = File::open(zip).map_err(|e| Error::Io(format!("{}: {e}", zip.display())))?;
    let list = entries(&mut file)?;
    let total: u64 = list.iter().map(|e| e.size).sum();
    if total > MAX_TOTAL {
        return Err(bad("archive unpacks to more than 16 GiB"));
    }
    let mut files = 0;
    for e in &list {
        let rel = e.name.trim_start_matches("./");
        if rel.is_empty() || !crate::manifest::safe_relative(rel) {
            return Err(bad(format!("entry {:?} would unpack outside the mod folder", e.name)));
        }
        let path = dest.join(rel);
        if e.is_dir() {
            std::fs::create_dir_all(&path)?;
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // The local header repeats the name and has its own extra field.
        file.seek(SeekFrom::Start(e.local_offset))?;
        let mut header = [0; 30];
        file.read_exact(&mut header)?;
        if u32_at(&header, 0) != LOCAL {
            return Err(bad(format!("{}: damaged local header", e.name)));
        }
        let skip = u16_at(&header, 26) as i64 + u16_at(&header, 28) as i64;
        file.seek(SeekFrom::Current(skip))?;
        let raw = (&mut file).take(e.compressed);
        let mut reader: Box<dyn Read> = match e.method {
            0 => Box::new(raw),
            8 => Box::new(flate2::read::DeflateDecoder::new(raw)),
            m => return Err(bad(format!("{}: compression method {m} is not supported (use deflate)", e.name))),
        };
        let mut out = File::create(&path)?;
        let mut crc = crc32fast::Hasher::new();
        let mut written = 0u64;
        let mut buf = vec![0; 64 * 1024];
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            written += n as u64;
            if written > e.size {
                return Err(bad(format!("{}: bigger than the archive says", e.name)));
            }
            crc.update(&buf[..n]);
            out.write_all(&buf[..n])?;
        }
        if written != e.size || crc.finalize() != e.crc {
            return Err(bad(format!("{}: damaged (size or checksum wrong)", e.name)));
        }
        files += 1;
    }
    Ok(files)
}

/// Writes simple zip files (tests and `p2mods pack`): stored or deflated entries.
pub struct ZipWriter {
    out: File,
    offset: u64,
    central: Vec<u8>,
    count: u16,
}

impl ZipWriter {
    pub fn create(path: &Path) -> Result<Self, Error> {
        Ok(ZipWriter { out: File::create(path)?, offset: 0, central: Vec::new(), count: 0 })
    }

    pub fn add(&mut self, name: &str, data: &[u8], deflate: bool) -> Result<(), Error> {
        let crc = crc32fast::hash(data);
        let body = if deflate {
            let mut enc = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
            enc.write_all(data)?;
            enc.finish()?
        } else {
            data.to_vec()
        };
        let method: u16 = if deflate { 8 } else { 0 };
        let name_b = name.as_bytes();
        let mut local = Vec::with_capacity(30 + name_b.len());
        local.extend_from_slice(&LOCAL.to_le_bytes());
        local.extend_from_slice(&20u16.to_le_bytes()); // version needed
        local.extend_from_slice(&0x0800u16.to_le_bytes()); // flags: UTF-8 names
        local.extend_from_slice(&method.to_le_bytes());
        local.extend_from_slice(&[0; 4]); // time, date
        local.extend_from_slice(&crc.to_le_bytes());
        local.extend_from_slice(&(body.len() as u32).to_le_bytes());
        local.extend_from_slice(&(data.len() as u32).to_le_bytes());
        local.extend_from_slice(&(name_b.len() as u16).to_le_bytes());
        local.extend_from_slice(&0u16.to_le_bytes());
        local.extend_from_slice(name_b);
        self.out.write_all(&local)?;
        self.out.write_all(&body)?;
        let c = &mut self.central;
        c.extend_from_slice(&CENTRAL.to_le_bytes());
        c.extend_from_slice(&20u16.to_le_bytes()); // made by
        c.extend_from_slice(&local[4..28]); // needed .. name length (the same fields)
        c.extend_from_slice(&0u16.to_le_bytes()); // extra
        c.extend_from_slice(&0u16.to_le_bytes()); // comment
        c.extend_from_slice(&0u16.to_le_bytes()); // disk
        c.extend_from_slice(&0u16.to_le_bytes()); // internal attributes
        c.extend_from_slice(&0u32.to_le_bytes()); // external attributes
        c.extend_from_slice(&(self.offset as u32).to_le_bytes());
        c.extend_from_slice(name_b);
        self.offset += (local.len() + body.len()) as u64;
        self.count += 1;
        Ok(())
    }

    pub fn finish(mut self) -> Result<(), Error> {
        self.out.write_all(&self.central)?;
        let mut end = Vec::with_capacity(22);
        end.extend_from_slice(&EOCD.to_le_bytes());
        end.extend_from_slice(&[0; 4]); // disk numbers
        end.extend_from_slice(&self.count.to_le_bytes());
        end.extend_from_slice(&self.count.to_le_bytes());
        end.extend_from_slice(&(self.central.len() as u32).to_le_bytes());
        end.extend_from_slice(&(self.offset as u32).to_le_bytes());
        end.extend_from_slice(&0u16.to_le_bytes());
        self.out.write_all(&end)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TempDir;

    #[test]
    fn round_trip_stored_and_deflated() {
        let t = TempDir::new("zip");
        let zip = t.path().join("a.zip");
        let mut w = ZipWriter::create(&zip).unwrap();
        let big: Vec<u8> = (0..100_000u32).map(|i| (i % 7) as u8).collect();
        w.add("mod.toml", b"id = \"a\"", false).unwrap();
        w.add("materials/", b"", false).unwrap();
        w.add("materials/x/big.vtf", &big, true).unwrap();
        w.finish().unwrap();
        let out = t.path().join("out");
        std::fs::create_dir(&out).unwrap();
        assert_eq!(extract(&zip, &out).unwrap(), 2);
        assert_eq!(std::fs::read(out.join("mod.toml")).unwrap(), b"id = \"a\"");
        assert_eq!(std::fs::read(out.join("materials/x/big.vtf")).unwrap(), big);
        // Deflate really compressed it.
        assert!(std::fs::metadata(&zip).unwrap().len() < 10_000);
    }

    #[test]
    fn refuses_entries_outside_the_folder() {
        let t = TempDir::new("zipslip");
        for name in ["../evil.txt", "/etc/evil", "a/../../evil"] {
            let zip = t.path().join("bad.zip");
            let mut w = ZipWriter::create(&zip).unwrap();
            w.add(name, b"x", false).unwrap();
            w.finish().unwrap();
            let out = t.path().join("out");
            let _ = std::fs::create_dir(&out);
            let e = extract(&zip, &out).unwrap_err().to_string();
            assert!(e.contains("outside"), "{name}: {e}");
        }
        assert!(!t.path().join("evil.txt").exists());
    }

    #[test]
    fn detects_damage_and_non_zips() {
        let t = TempDir::new("zipbad");
        let zip = t.path().join("a.zip");
        let mut w = ZipWriter::create(&zip).unwrap();
        w.add("f.txt", b"hello world", false).unwrap();
        w.finish().unwrap();
        let mut bytes = std::fs::read(&zip).unwrap();
        // Flip a byte of the stored data (after the 30-byte header and 5-byte name).
        bytes[36] ^= 0xFF;
        std::fs::write(&zip, &bytes).unwrap();
        let out = t.path().join("out");
        std::fs::create_dir(&out).unwrap();
        assert!(extract(&zip, &out).unwrap_err().to_string().contains("damaged"));
        std::fs::write(&zip, b"not a zip at all, just text").unwrap();
        assert!(extract(&zip, &out).unwrap_err().to_string().contains("not a zip"));
    }
}
