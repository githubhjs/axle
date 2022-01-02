#![no_std]

extern crate alloc;
use axle_rt::HasEventField;
use cstr_core::CString;

pub fn copy_str_into_sized_slice(slice: &mut [u8], s: &str) -> () {
    let c_str = CString::new(s).unwrap();
    let c_str_bytes = c_str.as_bytes_with_nul();
    slice[..c_str_bytes.len()].copy_from_slice(c_str_bytes);
}

pub fn str_from_u8_nul_utf8_unchecked(utf8_src: &[u8]) -> &str {
    let nul_range_end = utf8_src
        .iter()
        .position(|&c| c == b'\0')
        .unwrap_or(utf8_src.len()); // default to length if no `\0` present
    unsafe { core::str::from_utf8_unchecked(&utf8_src[0..nul_range_end]) }
}

pub const FILE_MANAGER_READ_DIRECTORY: u32 = 100;

#[repr(C)]
#[derive(Debug)]
pub struct FileManagerReadDirectory {
    pub event: u32,
    pub dir: [u8; 64],
}

impl FileManagerReadDirectory {
    pub fn new(dir: &str) -> Self {
        let mut s = FileManagerReadDirectory {
            event: FILE_MANAGER_READ_DIRECTORY,
            dir: [0; 64],
        };
        copy_str_into_sized_slice(&mut s.dir, dir);
        s
    }
}

impl axle_rt::HasEventField for FileManagerReadDirectory {
    fn event(&self) -> u32 {
        self.event
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FileManagerDirectoryEntry {
    pub name: [u8; 64],
    pub is_directory: bool,
}

impl FileManagerDirectoryEntry {
    pub fn new(name: &str, is_directory: bool) -> Self {
        let mut ret = FileManagerDirectoryEntry {
            name: [0; 64],
            is_directory,
        };
        copy_str_into_sized_slice(&mut ret.name, name);
        ret
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct FileManagerDirectoryContents {
    pub event: u32,
    pub entries: [Option<FileManagerDirectoryEntry>; 64],
}

impl HasEventField for FileManagerDirectoryContents {
    fn event(&self) -> u32 {
        self.event
    }
}
