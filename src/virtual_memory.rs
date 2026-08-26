
#[derive(Debug, Default, Clone)]
pub struct VirtualMemory {
    // TODO: remove pubs
    pub raw_pointer: *mut u8,
    pub size: usize,
    pub offset: usize,
    pub flags: u32,
}
