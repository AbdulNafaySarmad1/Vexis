//! Minimal PE64 loader: headers, sections, entry point.
//!
//! Uses `goblin` for header parsing only. All instruction-level work happens in
//! `crate::decoder`.

use goblin::pe::PE;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct Section {
    pub name: String,
    pub virtual_address: u64,
    pub virtual_size: u64,
    pub raw_size: u64,
    pub characteristics: u32,
    #[serde(skip)]
    pub data: Vec<u8>,
}

impl Section {
    pub const IMAGE_SCN_MEM_EXECUTE: u32 = 0x2000_0000;
    pub const IMAGE_SCN_CNT_CODE: u32 = 0x0000_0020;

    pub fn is_executable(&self) -> bool {
        self.characteristics & (Self::IMAGE_SCN_MEM_EXECUTE | Self::IMAGE_SCN_CNT_CODE) != 0
    }
    pub fn contains_va(&self, va: u64) -> bool {
        va >= self.virtual_address
            && va < self.virtual_address + self.virtual_size.max(self.raw_size)
    }
    /// Slice of section bytes starting at `va` (returns `None` if out of range).
    pub fn bytes_from(&self, va: u64) -> Option<&[u8]> {
        if !self.contains_va(va) {
            return None;
        }
        let off = (va - self.virtual_address) as usize;
        self.data.get(off..)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LoadedPe {
    pub path: String,
    pub is_64: bool,
    pub image_base: u64,
    /// Entry point as a virtual address (image_base + AddressOfEntryPoint).
    pub entry_va: u64,
    pub sections: Vec<Section>,
}

impl LoadedPe {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let bytes = std::fs::read(&path)?;
        Self::from_bytes(&bytes, path.as_ref().display().to_string())
    }

    pub fn from_bytes(bytes: &[u8], path: String) -> Result<Self, Box<dyn std::error::Error>> {
        let pe = PE::parse(bytes)?;
        let image_base = pe.image_base as u64;
        let entry_va = image_base + pe.entry as u64;

        let mut sections = Vec::new();
        for s in &pe.sections {
            let name = s
                .name()
                .unwrap_or("<invalid>")
                .trim_end_matches('\0')
                .to_string();
            let raw_start = s.pointer_to_raw_data as usize;
            let raw_len = s.size_of_raw_data as usize;
            let data = bytes
                .get(raw_start..raw_start.saturating_add(raw_len))
                .unwrap_or(&[])
                .to_vec();
            sections.push(Section {
                name,
                virtual_address: image_base + s.virtual_address as u64,
                virtual_size: s.virtual_size as u64,
                raw_size: raw_len as u64,
                characteristics: s.characteristics,
                data,
            });
        }

        Ok(LoadedPe {
            path,
            is_64: pe.is_64,
            image_base,
            entry_va,
            sections,
        })
    }

    pub fn section_for_va(&self, va: u64) -> Option<&Section> {
        self.sections.iter().find(|s| s.contains_va(va))
    }

    pub fn text_section(&self) -> Option<&Section> {
        self.sections
            .iter()
            .find(|s| s.name == ".text")
            .or_else(|| self.sections.iter().find(|s| s.is_executable()))
    }

    /// All executable sections, in image order.
    pub fn code_sections(&self) -> impl Iterator<Item = &Section> {
        self.sections.iter().filter(|s| s.is_executable())
    }
}
