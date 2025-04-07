/*
 * rustnix-fs
 * at the start of the disk, there is a superblock, which contains the following information:
 * - the size of the disk
 * - the size of the inode table
 * - the size of the data blocks
 * - the number of inodes
 * - the number of data blocks
 *
 * the superblock is followed by the inode table, which contains the following information:
 * - the size of the file
 * - the number of data blocks used by the file
 * - the data block pointers
 *
 * the inode table is followed by the data blocks, which contain the actual data of the files, including a metadata header that contains the following information:
 * - the owner of the file
 * - creation time
 * - modification time
 * - access time
 * - permissions, Unix-style
 *
 * Note that directories are just a figment of the filesystem's imagination; they are not implemented in this filesystem, rather being abstracted away by the virtal FS and the filenames. They cannot have properties like size, creation time, and can never be empty.
 */

use core::{default, fmt::Display};

use lazy_static::lazy_static;
use spin::Mutex;

#[allow(unused_imports)] // ALL_FLAGS is used
use crate::internal::{
    ata::{BLOCK_SIZE, read, write},
    clk,
    file::{ALL_FLAGS, FileError, FileFlags, FileSystem, Stream},
};

#[allow(unused_imports)] // warn is used
use log::{trace, warn};

use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use hashbrown::HashMap;

/// Magic number for our filesystem ("rustnix ")
pub const MAGIC_NUMBER: u64 = 0x727573746e697820;
/// Number of pointers in a block
pub const POINTERS_PER_BLOCK: usize = BLOCK_SIZE / 8; // 8: size of u64

lazy_static! {
    /// list of filesystems
    pub static ref FILESYSTEMS: Mutex<HashMap<(usize, usize), VirtFs>> = Mutex::new(HashMap::new());
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct Superblock {
    magic_number: u64,
    disk_size: u64,
    inode_table_size: u64,
    data_block_size: u64,
    num_inodes: u64,
    num_data_blocks: u64,
}

/// an inode
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Inode {
    num_data_blocks: u64,
    data_block_pointers: [u64; 12],
    // points to a block that contains pointers to data blocks
    single_indirect_block_pointer: u64,
    double_indirect_block_pointer: u64,
    triple_indirect_block_pointer: u64,
    file_name: [u8; 384],
}

/// A data block
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DataBlock {
    /// the data in the block
    pub data: [u8; 512],
}

/// Metadata for a file
#[derive(Debug, Clone)]
#[repr(C)]
pub struct FileMetadata {
    owner: u64,
    creation_time: u64,
    modification_time: u64,
    access_time: u64,
    permissions: u64, // Unix-style
}

/// A physical filesystem
#[derive(Debug, Clone)]
pub struct PhysFs {
    superblock: Superblock,
    /// the inode table
    pub inode_table: Vec<Inode>,
    /// the data blocks
    pub data_blocks: Vec<DataBlock>,
}

fn read_sector(bus: u8, dsk: u8, sector: u32) -> Result<Vec<u8>, ()> {
    let mut buf = vec![0; BLOCK_SIZE];
    read(bus, dsk, sector, &mut buf)?;
    Ok(buf)
}

impl default::Default for Inode {
    fn default() -> Self {
        Inode {
            num_data_blocks: 0,
            data_block_pointers: [0; 12],
            single_indirect_block_pointer: 0,
            double_indirect_block_pointer: 0,
            triple_indirect_block_pointer: 0,
            file_name: [0; 384],
        }
    }
}

/// list of filesystem errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsError {
    /// invalid path
    InvalidPath,
    /// file not found
    FileNotFound,
    /// file already exists
    FileExists,
    /// disk is full
    DiskFull,
    /// out of inodes
    OutOfInodes,
    /// out of data blocks
    OutOfDataBlocks,
    /// invalid inode
    InvalidInode,
    /// invalid data block
    InvalidDataBlock,
    /// invalid superblock
    InvalidSuperblock,
    /// invalid inode table
    InvalidInodeTable,
    /// invalid metadata
    InvalidMetadata,
    /// write error
    WriteError,
    /// read error
    ReadError,
    /// file is unwritable
    UnwritableFile,
    /// file is unreadable
    UnreadableFile,
    /// filesystem not found
    FilesystemNotFound,
    /// filesystem already exists
    FilesystemExists,
    /// invalid file descriptor
    InvalidFileDescriptor,
}

impl Display for FsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", match self {
            FsError::InvalidPath => "Invalid path".to_string(),
            FsError::FileNotFound => FsError::FileNotFound.to_string(),
            FsError::FileExists => "File already exists".to_string(),
            FsError::DiskFull => "Disk is full".to_string(),
            FsError::OutOfInodes => "Out of inodes".to_string(),
            FsError::OutOfDataBlocks => "Out of data blocks".to_string(),
            FsError::InvalidInode => "Invalid inode".to_string(),
            FsError::InvalidDataBlock => "Invalid data block".to_string(),
            FsError::InvalidSuperblock => "Invalid superblock".to_string(),
            FsError::InvalidInodeTable => "Invalid inode table".to_string(),
            FsError::InvalidMetadata => "Invalid metadata".to_string(),
            FsError::WriteError => "Write error".to_string(),
            FsError::ReadError => "Read error".to_string(),
            FsError::UnwritableFile => "File is unwritable".to_string(),
            FsError::UnreadableFile => "File is unreadable".to_string(),
            FsError::FilesystemNotFound => "Filesystem not found".to_string(),
            FsError::FilesystemExists => "Filesystem already exists".to_string(),
            FsError::InvalidFileDescriptor => "Invalid file descriptor".to_string(),
        })
    }
}

impl PhysFs {
    /// allocate a new block
    pub fn allocate_block(&mut self, inode_index: usize, block_num: u64) {
        let mut inode = self.inode_table[inode_index];
        if block_num < 12 {
            (&mut inode).data_block_pointers[block_num as usize] = block_num;
        } else if block_num < 12 + POINTERS_PER_BLOCK as u64 {
            self.allocate_single_indirect_block(&mut inode, block_num - 12);
        } else if block_num < 12 + POINTERS_PER_BLOCK as u64 * POINTERS_PER_BLOCK as u64 {
            self.allocate_double_indirect_block(
                &mut inode,
                block_num - 12 - POINTERS_PER_BLOCK as u64,
            );
        } else {
            self.allocate_triple_indirect_block(
                &mut inode,
                block_num - 12 - POINTERS_PER_BLOCK as u64 * POINTERS_PER_BLOCK as u64,
            );
        }
    }

    fn allocate_single_indirect_block(&mut self, inode: &mut Inode, block_num: u64) {
        if inode.single_indirect_block_pointer == 0 {
            inode.single_indirect_block_pointer = self
                .find_empty_data_block(None)
                .expect("Block allocation failed");
        }
        let block_index = (block_num % POINTERS_PER_BLOCK as u64) as usize;
        let block = &mut self.data_blocks[inode.single_indirect_block_pointer as usize];
        let pointers: &mut [u64; POINTERS_PER_BLOCK] =
            unsafe { &mut *(block.data.as_mut_ptr() as *mut [u64; POINTERS_PER_BLOCK]) };
        pointers[block_index] = self
            .find_empty_data_block(Some(pointers.to_vec()))
            .expect("Block allocation failed");
    }

    fn allocate_double_indirect_block(&mut self, inode: &mut Inode, block_num: u64) {
        if inode.double_indirect_block_pointer == 0 {
            inode.double_indirect_block_pointer = self
                .find_empty_data_block(None)
                .expect("Block allocation failed");
        }
        let block_index = (block_num / POINTERS_PER_BLOCK as u64) as usize;
        let block = &mut self.data_blocks[inode.double_indirect_block_pointer as usize];
        let pointers: &mut [u64; POINTERS_PER_BLOCK] =
            unsafe { &mut *(block.data.as_mut_ptr() as *mut [u64; POINTERS_PER_BLOCK]) };
        if pointers[block_index] == 0 {
            pointers[block_index] = self
                .find_empty_data_block(Some(pointers.to_vec()))
                .expect("Block allocation failed");
        }
        self.allocate_single_indirect_block(
            &mut Inode {
                single_indirect_block_pointer: pointers[block_index],
                ..Default::default()
            },
            block_num % POINTERS_PER_BLOCK as u64,
        );
    }

    fn allocate_triple_indirect_block(&mut self, inode: &mut Inode, block_num: u64) {
        if inode.triple_indirect_block_pointer == 0 {
            inode.triple_indirect_block_pointer = self
                .find_empty_data_block(None)
                .expect("Block allocation failed");
        }
        let block_index =
            (block_num / (POINTERS_PER_BLOCK as u64 * POINTERS_PER_BLOCK as u64)) as usize;
        let block = &mut self.data_blocks[inode.triple_indirect_block_pointer as usize];
        let pointers: &mut [u64; POINTERS_PER_BLOCK] =
            unsafe { &mut *(block.data.as_mut_ptr() as *mut [u64; POINTERS_PER_BLOCK]) };
        if pointers[block_index] == 0 {
            pointers[block_index] = self
                .find_empty_data_block(Some(pointers.to_vec()))
                .expect("Block allocation failed");
        }
        self.allocate_double_indirect_block(
            &mut Inode {
                double_indirect_block_pointer: pointers[block_index],
                ..Default::default()
            },
            block_num % (POINTERS_PER_BLOCK as u64 * POINTERS_PER_BLOCK as u64),
        );
    }

    /// get the block address for a given inode and block number
    pub fn get_block(&self, inode_index: usize, block_num: u64) -> u64 {
        let inode = &self.inode_table[inode_index];
        if block_num < 12 {
            inode.data_block_pointers[block_num as usize]
        } else if block_num < 12 + POINTERS_PER_BLOCK as u64 {
            self.get_single_indirect_block(inode, block_num - 12)
        } else if block_num < 12 + POINTERS_PER_BLOCK as u64 * POINTERS_PER_BLOCK as u64 {
            self.get_double_indirect_block(inode, block_num - 12 - POINTERS_PER_BLOCK as u64)
        } else {
            self.get_triple_indirect_block(
                inode,
                block_num - 12 - POINTERS_PER_BLOCK as u64 * POINTERS_PER_BLOCK as u64,
            )
        }
    }

    fn get_single_indirect_block(&self, inode: &Inode, block_num: u64) -> u64 {
        let block_index = (block_num % POINTERS_PER_BLOCK as u64) as usize;
        let block = &self.data_blocks[inode.single_indirect_block_pointer as usize];
        let pointers: &[u64; POINTERS_PER_BLOCK] =
            unsafe { &*(block.data.as_ptr() as *const [u64; POINTERS_PER_BLOCK]) };
        pointers[block_index]
    }

    fn get_double_indirect_block(&self, inode: &Inode, block_num: u64) -> u64 {
        let block_index = (block_num / POINTERS_PER_BLOCK as u64) as usize;
        let block = &self.data_blocks[inode.double_indirect_block_pointer as usize];
        let pointers: &[u64; POINTERS_PER_BLOCK] =
            unsafe { &*(block.data.as_ptr() as *const [u64; POINTERS_PER_BLOCK]) };
        self.get_single_indirect_block(
            &Inode {
                single_indirect_block_pointer: pointers[block_index],
                ..Default::default()
            },
            block_num % POINTERS_PER_BLOCK as u64,
        )
    }

    fn get_triple_indirect_block(&self, inode: &Inode, block_num: u64) -> u64 {
        let block_index =
            (block_num / (POINTERS_PER_BLOCK as u64 * POINTERS_PER_BLOCK as u64)) as usize;
        let block = &self.data_blocks[inode.triple_indirect_block_pointer as usize];
        let pointers: &[u64; POINTERS_PER_BLOCK] =
            unsafe { &*(block.data.as_ptr() as *const [u64; POINTERS_PER_BLOCK]) };
        self.get_double_indirect_block(
            &Inode {
                double_indirect_block_pointer: pointers[block_index],
                ..Default::default()
            },
            block_num % (POINTERS_PER_BLOCK as u64 * POINTERS_PER_BLOCK as u64),
        )
    }

    fn get_all_block_addresses(&self, inode: &Inode) -> Vec<u64> {
        let mut block_addresses = Vec::new();

        // Direct blocks
        for &block in &inode.data_block_pointers {
            if block != 0 {
                block_addresses.push(block);
            }
        }

        // Single indirect blocks
        if inode.single_indirect_block_pointer != 0 {
            let single_indirect_block =
                &self.data_blocks[inode.single_indirect_block_pointer as usize];
            let pointers: &[u64; POINTERS_PER_BLOCK] = unsafe {
                &*(single_indirect_block.data.as_ptr() as *const [u64; POINTERS_PER_BLOCK])
            };
            for &block in pointers {
                if block != 0 {
                    block_addresses.push(block);
                }
            }
        }

        // Double indirect blocks
        if inode.double_indirect_block_pointer != 0 {
            let double_indirect_block =
                &self.data_blocks[inode.double_indirect_block_pointer as usize];
            let pointers: &[u64; POINTERS_PER_BLOCK] = unsafe {
                &*(double_indirect_block.data.as_ptr() as *const [u64; POINTERS_PER_BLOCK])
            };
            for &single_indirect_pointer in pointers {
                if single_indirect_pointer != 0 {
                    let single_indirect_block = &self.data_blocks[single_indirect_pointer as usize];
                    let single_pointers: &[u64; POINTERS_PER_BLOCK] = unsafe {
                        &*(single_indirect_block.data.as_ptr() as *const [u64; POINTERS_PER_BLOCK])
                    };
                    for &block in single_pointers {
                        if block != 0 {
                            block_addresses.push(block);
                        }
                    }
                }
            }
        }

        // Triple indirect blocks
        if inode.triple_indirect_block_pointer != 0 {
            let triple_indirect_block =
                &self.data_blocks[inode.triple_indirect_block_pointer as usize];
            let pointers: &[u64; POINTERS_PER_BLOCK] = unsafe {
                &*(triple_indirect_block.data.as_ptr() as *const [u64; POINTERS_PER_BLOCK])
            };
            for &double_indirect_pointer in pointers {
                if double_indirect_pointer != 0 {
                    let double_indirect_block = &self.data_blocks[double_indirect_pointer as usize];
                    let double_pointers: &[u64; POINTERS_PER_BLOCK] = unsafe {
                        &*(double_indirect_block.data.as_ptr() as *const [u64; POINTERS_PER_BLOCK])
                    };
                    for &single_indirect_pointer in double_pointers {
                        if single_indirect_pointer != 0 {
                            let single_indirect_block =
                                &self.data_blocks[single_indirect_pointer as usize];
                            let single_pointers: &[u64; POINTERS_PER_BLOCK] = unsafe {
                                &*(single_indirect_block.data.as_ptr()
                                    as *const [u64; POINTERS_PER_BLOCK])
                            };
                            for &block in single_pointers {
                                if block != 0 {
                                    block_addresses.push(block);
                                }
                            }
                        }
                    }
                }
            }
        }

        block_addresses
    }

    fn read_from_disk(bus: usize, device: usize) -> Result<Self, FsError> {
        // read the superblock from the disk (it takes up the first sector)
        let sector_data =
            read_sector(bus as u8, device as u8, 0).map_err(|_| FsError::ReadError)?;
        let superblock = Superblock {
            magic_number: u64::from_le_bytes(
                sector_data[0..8]
                    .try_into()
                    .map_err(|_| FsError::InvalidSuperblock)?,
            ),
            disk_size: u64::from_le_bytes(
                sector_data[8..16]
                    .try_into()
                    .map_err(|_| FsError::InvalidSuperblock)?,
            ),
            inode_table_size: u64::from_le_bytes(
                sector_data[16..24]
                    .try_into()
                    .map_err(|_| FsError::InvalidSuperblock)?,
            ),
            data_block_size: u64::from_le_bytes(
                sector_data[24..32]
                    .try_into()
                    .map_err(|_| FsError::InvalidSuperblock)?,
            ),
            num_inodes: u64::from_le_bytes(
                sector_data[32..40]
                    .try_into()
                    .map_err(|_| FsError::InvalidSuperblock)?,
            ),
            num_data_blocks: u64::from_le_bytes(
                sector_data[40..48]
                    .try_into()
                    .map_err(|_| FsError::InvalidSuperblock)?,
            ),
        };

        if superblock.magic_number != MAGIC_NUMBER {
            return Err(FsError::InvalidSuperblock);
        }

        // read the inode table from the disk
        let mut inode_table = vec![
            Inode {
                num_data_blocks: 0,
                data_block_pointers: [0; 12],
                single_indirect_block_pointer: 0,
                double_indirect_block_pointer: 0,
                triple_indirect_block_pointer: 0,
                file_name: [0; 384],
            };
            superblock.num_inodes as usize
        ];

        for i in 0..superblock.inode_table_size {
            let sector_data = read_sector(bus as u8, device as u8, (1 + i) as u32)
                .map_err(|_| FsError::ReadError)?;
            inode_table[i as usize] = Inode {
                num_data_blocks: u64::from_le_bytes(
                    sector_data[0..8]
                        .try_into()
                        .map_err(|_| FsError::InvalidInode)?,
                ),
                data_block_pointers: [
                    u64::from_le_bytes(
                        sector_data[8..16]
                            .try_into()
                            .map_err(|_| FsError::InvalidInode)?,
                    ),
                    u64::from_le_bytes(
                        sector_data[16..24]
                            .try_into()
                            .map_err(|_| FsError::InvalidInode)?,
                    ),
                    u64::from_le_bytes(
                        sector_data[24..32]
                            .try_into()
                            .map_err(|_| FsError::InvalidInode)?,
                    ),
                    u64::from_le_bytes(
                        sector_data[32..40]
                            .try_into()
                            .map_err(|_| FsError::InvalidInode)?,
                    ),
                    u64::from_le_bytes(
                        sector_data[40..48]
                            .try_into()
                            .map_err(|_| FsError::InvalidInode)?,
                    ),
                    u64::from_le_bytes(
                        sector_data[48..56]
                            .try_into()
                            .map_err(|_| FsError::InvalidInode)?,
                    ),
                    u64::from_le_bytes(
                        sector_data[56..64]
                            .try_into()
                            .map_err(|_| FsError::InvalidInode)?,
                    ),
                    u64::from_le_bytes(
                        sector_data[64..72]
                            .try_into()
                            .map_err(|_| FsError::InvalidInode)?,
                    ),
                    u64::from_le_bytes(
                        sector_data[72..80]
                            .try_into()
                            .map_err(|_| FsError::InvalidInode)?,
                    ),
                    u64::from_le_bytes(
                        sector_data[80..88]
                            .try_into()
                            .map_err(|_| FsError::InvalidInode)?,
                    ),
                    u64::from_le_bytes(
                        sector_data[88..96]
                            .try_into()
                            .map_err(|_| FsError::InvalidInode)?,
                    ),
                    u64::from_le_bytes(
                        sector_data[96..104]
                            .try_into()
                            .map_err(|_| FsError::InvalidInode)?,
                    ),
                ],
                single_indirect_block_pointer: u64::from_le_bytes(
                    sector_data[104..112]
                        .try_into()
                        .map_err(|_| FsError::InvalidInode)?,
                ),
                double_indirect_block_pointer: u64::from_le_bytes(
                    sector_data[112..120]
                        .try_into()
                        .map_err(|_| FsError::InvalidInode)?,
                ),
                triple_indirect_block_pointer: u64::from_le_bytes(
                    sector_data[120..128]
                        .try_into()
                        .map_err(|_| FsError::InvalidInode)?,
                ),
                file_name: sector_data[128..512]
                    .try_into()
                    .map_err(|_| FsError::InvalidInode)?,
            };
        }

        // read the data blocks from the disk
        let mut data_blocks =
            vec![DataBlock { data: [0; 512] }; superblock.num_data_blocks as usize];

        for i in 0..superblock.num_data_blocks {
            let sector_data = read_sector(
                bus as u8,
                device as u8,
                (1 + superblock.inode_table_size + i) as u32, // superblock + inode table
            )
            .map_err(|_| FsError::ReadError)?;
            data_blocks[i as usize] = DataBlock {
                data: sector_data
                    .try_into()
                    .map_err(|_| FsError::InvalidDataBlock)?,
            };
        }

        Ok(PhysFs {
            superblock: superblock.clone(),
            inode_table,
            data_blocks,
        })
    }

    /// write the filesystem to the disk
    pub fn write_to_disk(&self, bus: usize, device: usize) -> Result<(), FsError> {
        // write the superblock to the disk
        let mut sector_data = vec![0; BLOCK_SIZE];
        sector_data[0..8].copy_from_slice(&self.superblock.magic_number.to_le_bytes());
        sector_data[8..16].copy_from_slice(&self.superblock.disk_size.to_le_bytes());
        sector_data[16..24].copy_from_slice(&self.superblock.inode_table_size.to_le_bytes());
        sector_data[24..32].copy_from_slice(&self.superblock.data_block_size.to_le_bytes());
        sector_data[32..40].copy_from_slice(&self.superblock.num_inodes.to_le_bytes());
        sector_data[40..48].copy_from_slice(&self.superblock.num_data_blocks.to_le_bytes());
        write(bus as u8, device as u8, 0, &sector_data).map_err(|_| FsError::WriteError)?;

        // write the inode table to the disk
        for i in 0..self.superblock.inode_table_size {
            let mut sector_data = vec![0; BLOCK_SIZE];
            sector_data[0..8]
                .copy_from_slice(&self.inode_table[i as usize].num_data_blocks.to_le_bytes());
            for j in 0..12 {
                sector_data[j as usize * 8 + 8..j as usize * 8 + 16].copy_from_slice(
                    &self.inode_table[i as usize].data_block_pointers[j].to_le_bytes(),
                );
            }

            sector_data[104..112].copy_from_slice(
                &self.inode_table[i as usize]
                    .single_indirect_block_pointer
                    .to_le_bytes(),
            );
            sector_data[112..120].copy_from_slice(
                &self.inode_table[i as usize]
                    .double_indirect_block_pointer
                    .to_le_bytes(),
            );
            sector_data[120..128].copy_from_slice(
                &self.inode_table[i as usize]
                    .triple_indirect_block_pointer
                    .to_le_bytes(),
            );
            sector_data[128..512].copy_from_slice(&self.inode_table[i as usize].file_name);

            write(bus as u8, device as u8, 1 + i as u32, &sector_data)
                .map_err(|_| FsError::WriteError)?;
        }

        // write the data blocks to the disk
        for i in 0..self.superblock.num_data_blocks {
            let mut sector_data = vec![0; BLOCK_SIZE];
            sector_data.copy_from_slice(&self.data_blocks[i as usize].data);
            write(
                bus as u8,
                device as u8,
                (1 + self.superblock.inode_table_size + i) as u32,
                &sector_data,
            )
            .map_err(|_| FsError::WriteError)?;
        }

        Ok(())
    }

    fn find_empty_data_block(&self, ignore: Option<Vec<u64>>) -> Result<u64, FsError> {
        for i in 1..self.superblock.num_data_blocks {
            if self.data_blocks[i as usize].data == [0; 512]
                && !ignore.as_ref().map_or(false, |v| v.contains(&i))
            {
                // possibility that it is used, and just happens to be empty
                // check if it is actually used
                let mut used = false;
                for inode in &self.inode_table {
                    if inode.data_block_pointers.contains(&i) {
                        used = true;
                        break;
                    }
                }

                if used {
                    continue;
                } else {
                    return Ok(i);
                }
            }
        }

        Err(FsError::DiskFull)
    }

    fn find_empty_inode(&self) -> Result<u64, FsError> {
        for i in 0..self.superblock.num_inodes {
            if self.inode_table[i as usize].num_data_blocks == 0 {
                return Ok(i);
            }
        }

        Err(FsError::OutOfInodes)
    }

    fn write_to_data_block(&mut self, data_block: u64, data: &[u8]) -> Result<(), FsError> {
        if data_block >= self.superblock.num_data_blocks {
            return Err(FsError::InvalidDataBlock);
        }

        self.data_blocks[data_block as usize].data[..data.len()].copy_from_slice(data);
        Ok(())
    }

    fn create_inode(&mut self, inode: Inode) -> Result<(), FsError> {
        let inode_index = self.find_empty_inode()?;
        self.inode_table[inode_index as usize] = inode;
        Ok(())
    }

    fn update_inode(&mut self, inode: Inode) -> Result<(), FsError> {
        let inode_index = self
            .inode_table
            .iter()
            .position(|i| i.file_name == inode.file_name)
            .ok_or(FsError::InvalidInode)?;

        self.inode_table[inode_index as usize] = inode;
        Ok(())
    }

    fn create_file(&mut self, file_name: &str, perms: [u8; 3], owner: u64) -> Result<(), FsError> {
        let mut inode = Inode {
            num_data_blocks: 1,
            data_block_pointers: [0; 12],
            single_indirect_block_pointer: 0,
            double_indirect_block_pointer: 0,
            triple_indirect_block_pointer: 0,
            file_name: [0; 384],
        };

        let data_block = self.find_empty_data_block(None)?;
        inode.data_block_pointers[0] = data_block;
        inode.file_name[..file_name.len()].copy_from_slice(file_name.as_bytes());

        let metadata = FileMetadata {
            owner,
            creation_time: clk::get_unix_time(),
            modification_time: clk::get_unix_time(),
            access_time: clk::get_unix_time(),
            permissions: u64::from_le_bytes([perms[0], perms[1], perms[2], 0, 0, 0, 0, 0]),
        };

        let mut metadata_block = vec![0u8; 512];
        // cast the metadata to bytes and copy it into the metadata block
        metadata_block[..8].copy_from_slice(&metadata.owner.to_le_bytes());
        metadata_block[8..16].copy_from_slice(&metadata.creation_time.to_le_bytes());
        metadata_block[16..24].copy_from_slice(&metadata.modification_time.to_le_bytes());
        metadata_block[24..32].copy_from_slice(&metadata.access_time.to_le_bytes());
        metadata_block[32..40].copy_from_slice(&metadata.permissions.to_le_bytes());

        self.write_to_data_block(data_block, &metadata_block)?;

        self.create_inode(inode)?;
        Ok(())
    }

    fn read_file(&self, file_name: &str) -> Result<(Vec<u8>, FileMetadata), FsError> {
        let inode = self.find_inode_by_name(file_name)?;

        // the first data block contains the metadata
        let metadata_block = &self.data_blocks[inode.data_block_pointers[0] as usize].data;
        let metadata = FileMetadata {
            owner: u64::from_le_bytes(
                metadata_block[..8]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
            creation_time: u64::from_le_bytes(
                metadata_block[8..16]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
            modification_time: u64::from_le_bytes(
                metadata_block[16..24]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
            access_time: u64::from_le_bytes(
                metadata_block[24..32]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
            permissions: u64::from_le_bytes(
                metadata_block[32..40]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
        };

        let inode_pointers = self.get_all_block_addresses(&inode);

        let mut data = Vec::new();
        for i in 1..inode.num_data_blocks {
            let data_block = &self.data_blocks[inode_pointers[i as usize] as usize].data;
            data.extend_from_slice(data_block);
        }

        // trace!("data: {:?}", data[0..10].to_vec());

        Ok((data, metadata))
    }

    fn find_inode_by_name(&self, file_name: &str) -> Result<Inode, FsError> {
        fn pad_end(arr: &[u8]) -> [u8; 384] {
            let mut padded = [0; 384];
            padded[..arr.len()].copy_from_slice(arr);
            padded
        }

        self.inode_table
            .iter()
            .find(|i| i.file_name == pad_end(file_name.as_bytes()))
            .cloned()
            .ok_or(FsError::FileNotFound)
    }

    fn write_file(
        &mut self,
        file_name: &str,
        data: &[u8],
        perms: Option<[u8; 3]>,
        owner: Option<u64>,
    ) -> Result<(), FsError> {
        // first, pad out data to be a multiple of 512 bytes
        let mut data = data.to_vec();
        let padding = 512 - (data.len() % 512);
        data.resize(data.len() + padding, 0);

        let inode = self.find_inode_by_name(file_name)?;

        // write in the metadata block
        let mut existing_metadata_block = vec![0u8; 512];
        existing_metadata_block
            .copy_from_slice(&self.data_blocks[inode.data_block_pointers[0] as usize].data);

        let existing_metadata = FileMetadata {
            owner: u64::from_le_bytes(
                existing_metadata_block[..8]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
            creation_time: u64::from_le_bytes(
                existing_metadata_block[8..16]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
            modification_time: u64::from_le_bytes(
                existing_metadata_block[16..24]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
            access_time: u64::from_le_bytes(
                existing_metadata_block[24..32]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
            permissions: u64::from_le_bytes(
                existing_metadata_block[32..40]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
        };

        let metadata = FileMetadata {
            owner: owner.unwrap_or(existing_metadata.owner),
            creation_time: existing_metadata.creation_time,
            modification_time: clk::get_unix_time(),
            access_time: existing_metadata.access_time,
            permissions: perms.map_or(existing_metadata.permissions, |p| {
                u64::from_le_bytes([p[0], p[1], p[2], 0, 0, 0, 0, 0])
            }),
        };

        // write the metadata back
        let mut metadata_block = vec![0u8; 512];
        metadata_block[..8].copy_from_slice(&metadata.owner.to_le_bytes());
        metadata_block[8..16].copy_from_slice(&metadata.creation_time.to_le_bytes());
        metadata_block[16..24].copy_from_slice(&metadata.modification_time.to_le_bytes());
        metadata_block[24..32].copy_from_slice(&metadata.access_time.to_le_bytes());
        metadata_block[32..40].copy_from_slice(&metadata.permissions.to_le_bytes());

        self.write_to_data_block(inode.data_block_pointers[0], &metadata_block)?;

        // determine how many data blocks we need
        fn ceil(f: f64) -> u64 {
            match f as u64 {
                f2 if f2 as f64 == f => f2,
                f2 => f2 + 1,
            }
        }

        let num_data_blocks = ceil(data.len() as f64 / 512.0);
        let mut data_blocks_pointers = vec![0u64; (num_data_blocks) as usize];

        data_blocks_pointers[0] = inode.data_block_pointers[0];

        let mut existing_data_blocks = self.get_all_block_addresses(&inode);

        // remove the metadata block (first block)
        existing_data_blocks.remove(0);

        for i in 0..num_data_blocks {
            if (i as usize) < existing_data_blocks.len() {
                data_blocks_pointers[i as usize] = existing_data_blocks[i as usize];
            } else {
                let data_block = self.find_empty_data_block(Some(data_blocks_pointers.clone()))?;
                data_blocks_pointers[i as usize] = data_block;
            }
        }

        // now add the data to the data blocks
        for i in 0..num_data_blocks {
            self.write_to_data_block(
                data_blocks_pointers[i as usize],
                &data[(i as usize * 512)..((i + 1) as usize * 512)],
            )?;
        }

        // now update the inode with the new data block pointers
        let mut updated_inode = inode.clone();
        updated_inode.num_data_blocks = (data_blocks_pointers.len() + 1) as u64; // metadata block

        self.update_inode(updated_inode)?;

        for i in 0..data_blocks_pointers.len() {
            // add the data block pointers to the inode
            // i + 1 because the first block is the metadata block
            if (i + 1) < 12 {
                updated_inode.data_block_pointers[i + 1] = data_blocks_pointers[i];
            } else if (i + 1) < 12 + POINTERS_PER_BLOCK {
                self.allocate_single_indirect_block(&mut updated_inode, (i + 1) as u64 - 12);
                let block_index = ((i + 1) % POINTERS_PER_BLOCK) as usize;
                let block =
                    &mut self.data_blocks[updated_inode.single_indirect_block_pointer as usize];
                let pointers: &mut [u64; POINTERS_PER_BLOCK] =
                    unsafe { &mut *(block.data.as_mut_ptr() as *mut [u64; POINTERS_PER_BLOCK]) };
                pointers[block_index] = data_blocks_pointers[i];
            } else if (i + 1) < 12 + POINTERS_PER_BLOCK * POINTERS_PER_BLOCK {
                self.allocate_double_indirect_block(
                    &mut updated_inode,
                    (i + 1) as u64 - 12 - POINTERS_PER_BLOCK as u64,
                );
                let block_index = (i + 1) / POINTERS_PER_BLOCK;
                let block =
                    &mut self.data_blocks[updated_inode.double_indirect_block_pointer as usize];
                let pointers: &mut [u64; POINTERS_PER_BLOCK] =
                    unsafe { &mut *(block.data.as_mut_ptr() as *mut [u64; POINTERS_PER_BLOCK]) };
                if pointers[block_index] == 0 {
                    pointers[block_index] = self
                        .find_empty_data_block(Some(pointers.to_vec()))
                        .expect("Block allocation failed");
                }
                self.allocate_single_indirect_block(
                    &mut Inode {
                        single_indirect_block_pointer: pointers[block_index],
                        ..Default::default()
                    },
                    (i + 1) as u64 % POINTERS_PER_BLOCK as u64,
                );
                let single_indirect_block = &mut self.data_blocks[pointers[block_index] as usize];
                let single_pointers: &mut [u64; POINTERS_PER_BLOCK] = unsafe {
                    &mut *(single_indirect_block.data.as_mut_ptr()
                        as *mut [u64; POINTERS_PER_BLOCK])
                };
                single_pointers[((i + 1) as u64 % POINTERS_PER_BLOCK as u64) as usize] =
                    data_blocks_pointers[i];
            } else {
                self.allocate_triple_indirect_block(
                    &mut updated_inode,
                    (i + 1) as u64 - 12 - POINTERS_PER_BLOCK as u64 * POINTERS_PER_BLOCK as u64,
                );
                let block_index = ((i as u64 + 1)
                    / (POINTERS_PER_BLOCK as u64 * POINTERS_PER_BLOCK as u64))
                    as usize;
                let block =
                    &mut self.data_blocks[updated_inode.triple_indirect_block_pointer as usize];

                let pointers: &mut [u64; POINTERS_PER_BLOCK] =
                    unsafe { &mut *(block.data.as_mut_ptr() as *mut [u64; POINTERS_PER_BLOCK]) };
                if pointers[block_index] == 0 {
                    pointers[block_index] = self
                        .find_empty_data_block(Some(pointers.to_vec()))
                        .expect("Block allocation failed");
                }
                self.allocate_double_indirect_block(
                    &mut Inode {
                        double_indirect_block_pointer: pointers[block_index],
                        ..Default::default()
                    },
                    (i + 1) as u64 % (POINTERS_PER_BLOCK as u64 * POINTERS_PER_BLOCK as u64),
                );
                let double_indirect_block = &mut self.data_blocks[pointers[block_index] as usize];
                let double_pointers: &mut [u64; POINTERS_PER_BLOCK] = unsafe {
                    &mut *(double_indirect_block.data.as_mut_ptr()
                        as *mut [u64; POINTERS_PER_BLOCK])
                };
                self.allocate_single_indirect_block(
                    &mut Inode {
                        single_indirect_block_pointer: double_pointers[((i + 1) as u64
                            % (POINTERS_PER_BLOCK as u64 * POINTERS_PER_BLOCK as u64))
                            as usize],
                        ..Default::default()
                    },
                    (i + 1) as u64 % POINTERS_PER_BLOCK as u64,
                );
                let single_indirect_block = &mut self.data_blocks[double_pointers[((i + 1) as u64
                    % (POINTERS_PER_BLOCK as u64 * POINTERS_PER_BLOCK as u64))
                    as usize]
                    as usize];
                let single_pointers: &mut [u64; POINTERS_PER_BLOCK] = unsafe {
                    &mut *(single_indirect_block.data.as_mut_ptr()
                        as *mut [u64; POINTERS_PER_BLOCK])
                };
                single_pointers[((i + 1) as u64 % POINTERS_PER_BLOCK as u64) as usize] =
                    data_blocks_pointers[i];
            }
        }

        self.update_inode(updated_inode)?;

        Ok(())
    }
}

/// the exposed API for the filesystem, which implements File
#[derive(Debug, Clone)]
pub struct VirtFs {
    /// the physical filesystem
    pub phys_fs: PhysFs,
    bus: usize,
    dsk: usize,

    open_files: Vec<FileHandle>,
}

impl VirtFs {
    /// load the filesystem from a disk
    pub fn from_disk(bus: usize, dsk: usize) -> Result<(), FsError> {
        let phys_fs = PhysFs::read_from_disk(bus, dsk)?;

        let mut file_systems = FILESYSTEMS.lock();
        file_systems.insert((bus, dsk), VirtFs {
            phys_fs,
            bus,
            dsk,
            open_files: Vec::new(),
        });

        Ok(())
    }

    /// create a new filesystem with a given size, on a given bus and device (note that this call will NOT format the disk, instead the first flush call will)
    pub fn new(bus: usize, dsk: usize, disk_size: u64) {
        let mut file_systems = FILESYSTEMS.lock();
        file_systems.insert((bus, dsk), VirtFs {
            phys_fs: PhysFs {
                superblock: Superblock {
                    magic_number: MAGIC_NUMBER,
                    disk_size,
                    inode_table_size: 1024,
                    data_block_size: 512,
                    num_inodes: 1024,
                    num_data_blocks: (disk_size / 512) - 1024 - 1, // superblock + inode table
                },
                inode_table: vec![
                    Inode {
                        num_data_blocks: 0,
                        data_block_pointers: [0; 12],
                        single_indirect_block_pointer: 0,
                        double_indirect_block_pointer: 0,
                        triple_indirect_block_pointer: 0,
                        file_name: [0; 384],
                    };
                    1024
                ],
                data_blocks: vec![DataBlock { data: [0; 512] }; 1024],
            },
            bus: bus,
            dsk,
            open_files: Vec::new(),
        });
    }
}

/// the handle to a file, which implements Stream
#[derive(Debug, Clone)]
pub struct FileHandle {
    file_name: String,
    bus: usize,
    dsk: usize,
    flags: u8,
    file_pos: usize,
}

impl FileHandle {
    /// create a new file handle with explicit bus and device
    pub fn new(file_name: String, bus: usize, dsk: usize, flags: u8) -> Self {
        FileHandle {
            file_name,
            bus,
            dsk,
            flags,
            file_pos: 0,
        }
    }

    /// create a new file handle with the likely filesystem
    pub fn new_with_likely_fs(file_name: String, flags: u8) -> Result<Self, FileError> {
        let file_systems = FILESYSTEMS.lock();
        for (key, fs) in file_systems.iter() {
            if fs.phys_fs.find_inode_by_name(&file_name).is_ok() {
                return Ok(FileHandle {
                    file_name,
                    bus: key.0,
                    dsk: key.1,
                    flags,
                    file_pos: 0,
                });
            }
        }

        Err(FileError::NotFoundError(FsError::FileNotFound.into()))
    }
}

impl Stream for FileHandle {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, FileError> {
        let mut file_systems = FILESYSTEMS.lock();
        let fs = file_systems
            .get_mut(&(self.bus, self.dsk))
            .ok_or(FileError::NotFoundError(FsError::FilesystemNotFound.into()))?;

        if !(self.flags & (FileFlags::Read as u8) != 0) {
            return Err(FileError::PermissionError(FsError::ReadError.into()));
        }

        let (data, _) = fs.phys_fs.read_file(&self.file_name)?;

        // we know data will be a multiple of 512 bytes

        let len = buf.len().min(data.len() - self.file_pos);

        buf[..len].copy_from_slice(&data[self.file_pos..self.file_pos + len]);

        self.file_pos += len;

        Ok(len)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, FileError> {
        let mut file_systems = FILESYSTEMS.lock();
        let fs = file_systems
            .get_mut(&(self.bus, self.dsk))
            .ok_or(FileError::NotFoundError(FsError::FilesystemNotFound.into()))?;

        if !(self.flags & (FileFlags::Write as u8) != 0) {
            return Err(FileError::PermissionError(FsError::WriteError.into()));
        }

        let (mut data, _) = fs.phys_fs.read_file(&self.file_name)?;

        if self.file_pos > data.len() {
            data.resize(self.file_pos, 0); // Pad with zeros if seeking beyond EOF
        }

        if self.file_pos + buf.len() > data.len() {
            data.resize(self.file_pos + buf.len(), 0);
        }

        data[self.file_pos..self.file_pos + buf.len()].copy_from_slice(buf);
        fs.phys_fs.write_file(&self.file_name, &data, None, None)?;

        self.file_pos += buf.len();
        Ok(buf.len())
    }

    fn close(&mut self) -> Result<(), FileError> {
        let mut file_systems = FILESYSTEMS.lock();
        let fs = file_systems
            .get_mut(&(self.bus, self.dsk))
            .ok_or(FileError::NotFoundError(FsError::FilesystemNotFound.into()))?;
        fs.open_files.retain(|f| f.file_name != self.file_name);
        Ok(())
    }

    fn flush(&mut self) -> Result<(), FileError> {
        let file_systems = FILESYSTEMS.lock();
        let fs = file_systems
            .get(&(self.bus, self.dsk))
            .ok_or(FileError::NotFoundError(FsError::FilesystemNotFound.into()))?;
        fs.phys_fs.write_to_disk(fs.bus, fs.dsk)?;
        Ok(())
    }

    fn poll(&mut self, event: super::file::IOEvent) -> bool {
        match event {
            super::file::IOEvent::Read => !(self.flags & (FileFlags::Read as u8) != 0),
            super::file::IOEvent::Write => !(self.flags & (FileFlags::Write as u8) != 0),
        }
    }

    fn seek(&mut self, pos: usize) -> Result<usize, FileError> {
        self.file_pos = pos;
        Ok(pos)
    }
}

impl FileSystem for VirtFs {
    fn open(&mut self, path: &str, flags: u8) -> Result<Box<dyn Stream>, FileError> {
        let mut file_systems = FILESYSTEMS.lock();
        let fs = file_systems
            .get_mut(&(self.bus, self.dsk))
            .ok_or(FileError::NotFoundError(FsError::FilesystemNotFound.into()))?;
        let res = fs.phys_fs.read_file(path);
        if res.is_err() {
            if flags & (FileFlags::Create as u8) != 0 {
                fs.phys_fs.create_file(path, [6, 6, 6], 0)?;
            } else {
                return Err(FileError::NotFoundError(FsError::FileNotFound.into()));
            }
        }

        // if the append flag is set, seek to the end of the file
        let mut file_handle = FileHandle {
            file_name: path.to_string(),
            bus: self.bus,
            dsk: self.dsk,
            flags,
            file_pos: 0,
        };

        if flags & (FileFlags::Append as u8) != 0 {
            let (data, _) = fs.phys_fs.read_file(path)?;
            file_handle.file_pos = data.len();
        }

        Ok(Box::new(file_handle))
    }

    fn delete(&mut self, path: &str) -> Result<(), FileError> {
        let mut file_systems = FILESYSTEMS.lock();
        let fs = file_systems
            .get_mut(&(self.bus, self.dsk))
            .ok_or(FileError::NotFoundError(FsError::FilesystemNotFound.into()))?;
        let inode = fs.phys_fs.find_inode_by_name(path)?;
        for i in 0..inode.num_data_blocks {
            fs.phys_fs.data_blocks[inode.data_block_pointers[i as usize] as usize] =
                DataBlock { data: [0; 512] };
        }
        fs.phys_fs
            .inode_table
            .retain(|i| i.file_name != inode.file_name);
        Ok(())
    }

    fn exists(&mut self, path: &str) -> bool {
        let file_systems = FILESYSTEMS.lock();
        let fs = file_systems.get(&(self.bus, self.dsk)).unwrap();
        fs.phys_fs.find_inode_by_name(path).is_ok()
    }

    fn chmod(&mut self, path: &str, perms: [u8; 3]) -> Result<(), FileError> {
        let mut file_systems = FILESYSTEMS.lock();
        let fs = file_systems
            .get_mut(&(self.bus, self.dsk))
            .ok_or(FileError::NotFoundError(FsError::FilesystemNotFound.into()))?;
        let inode = fs.phys_fs.find_inode_by_name(path)?;
        let mut updated_inode = inode.clone();
        updated_inode.file_name = inode.file_name;
        updated_inode.num_data_blocks = inode.num_data_blocks;
        updated_inode.data_block_pointers = inode.data_block_pointers;
        updated_inode.single_indirect_block_pointer = inode.single_indirect_block_pointer;
        updated_inode.double_indirect_block_pointer = inode.double_indirect_block_pointer;
        updated_inode.triple_indirect_block_pointer = inode.triple_indirect_block_pointer;

        let metadata_block = &fs.phys_fs.data_blocks[inode.data_block_pointers[0] as usize].data;
        let mut metadata = FileMetadata {
            owner: u64::from_le_bytes(
                metadata_block[..8]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
            creation_time: u64::from_le_bytes(
                metadata_block[8..16]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
            modification_time: u64::from_le_bytes(
                metadata_block[16..24]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
            access_time: u64::from_le_bytes(
                metadata_block[24..32]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
            permissions: u64::from_le_bytes(
                metadata_block[32..40]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
        };

        metadata.permissions = u64::from_le_bytes([perms[0], perms[1], perms[2], 0, 0, 0, 0, 0]);

        let mut metadata_block = vec![0u8; 512];
        metadata_block[..8].copy_from_slice(&metadata.owner.to_le_bytes());
        metadata_block[8..16].copy_from_slice(&metadata.creation_time.to_le_bytes());
        metadata_block[16..24].copy_from_slice(&metadata.modification_time.to_le_bytes());
        metadata_block[24..32].copy_from_slice(&metadata.access_time.to_le_bytes());
        metadata_block[32..40].copy_from_slice(&metadata.permissions.to_le_bytes());

        fs.phys_fs
            .write_to_data_block(inode.data_block_pointers[0], &metadata_block)?;
        fs.phys_fs.update_inode(updated_inode)?;
        Ok(())
    }

    fn chown(&mut self, path: &str, owner: u64) -> Result<(), FileError> {
        let mut file_systems = FILESYSTEMS.lock();
        let fs = file_systems
            .get_mut(&(self.bus, self.dsk))
            .ok_or(FileError::NotFoundError(FsError::FilesystemNotFound.into()))?;
        let inode = fs.phys_fs.find_inode_by_name(path)?;
        let mut updated_inode = inode.clone();
        updated_inode.file_name = inode.file_name;
        updated_inode.num_data_blocks = inode.num_data_blocks;
        updated_inode.data_block_pointers = inode.data_block_pointers;
        updated_inode.single_indirect_block_pointer = inode.single_indirect_block_pointer;
        updated_inode.double_indirect_block_pointer = inode.double_indirect_block_pointer;
        updated_inode.triple_indirect_block_pointer = inode.triple_indirect_block_pointer;

        let metadata_block = &fs.phys_fs.data_blocks[inode.data_block_pointers[0] as usize].data;
        let mut metadata = FileMetadata {
            owner: u64::from_le_bytes(
                metadata_block[..8]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
            creation_time: u64::from_le_bytes(
                metadata_block[8..16]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
            modification_time: u64::from_le_bytes(
                metadata_block[16..24]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
            access_time: u64::from_le_bytes(
                metadata_block[24..32]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
            permissions: u64::from_le_bytes(
                metadata_block[32..40]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
        };

        metadata.owner = owner;

        let mut metadata_block = vec![0u8; 512];
        metadata_block[..8].copy_from_slice(&metadata.owner.to_le_bytes());
        metadata_block[8..16].copy_from_slice(&metadata.creation_time.to_le_bytes());
        metadata_block[16..24].copy_from_slice(&metadata.modification_time.to_le_bytes());
        metadata_block[24..32].copy_from_slice(&metadata.access_time.to_le_bytes());
        metadata_block[32..40].copy_from_slice(&metadata.permissions.to_le_bytes());

        fs.phys_fs
            .write_to_data_block(inode.data_block_pointers[0], &metadata_block)?;
        fs.phys_fs.update_inode(updated_inode)?;

        Ok(())
    }

    fn get_owner(&mut self, path: &str) -> Result<u64, FileError> {
        let file_systems = FILESYSTEMS.lock();
        let fs = file_systems
            .get(&(self.bus, self.dsk))
            .ok_or(FileError::NotFoundError(FsError::FilesystemNotFound.into()))?;
        let inode = fs.phys_fs.find_inode_by_name(path)?;
        let metadata_block = &fs.phys_fs.data_blocks[inode.data_block_pointers[0] as usize].data;
        let metadata = FileMetadata {
            owner: u64::from_le_bytes(
                metadata_block[..8]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
            creation_time: u64::from_le_bytes(
                metadata_block[8..16]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
            modification_time: u64::from_le_bytes(
                metadata_block[16..24]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
            access_time: u64::from_le_bytes(
                metadata_block[24..32]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
            permissions: u64::from_le_bytes(
                metadata_block[32..40]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
        };
        Ok(metadata.owner)
    }

    fn get_perms(&mut self, path: &str) -> Result<[u8; 3], FileError> {
        let file_systems = FILESYSTEMS.lock();
        let fs = file_systems
            .get(&(self.bus, self.dsk))
            .ok_or(FileError::NotFoundError(FsError::FilesystemNotFound.into()))?;
        let inode = fs.phys_fs.find_inode_by_name(path)?;
        let metadata_block = &fs.phys_fs.data_blocks[inode.data_block_pointers[0] as usize].data;
        let metadata = FileMetadata {
            owner: u64::from_le_bytes(
                metadata_block[..8]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
            creation_time: u64::from_le_bytes(
                metadata_block[8..16]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
            modification_time: u64::from_le_bytes(
                metadata_block[16..24]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
            access_time: u64::from_le_bytes(
                metadata_block[24..32]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
            permissions: u64::from_le_bytes(
                metadata_block[32..40]
                    .try_into()
                    .map_err(|_| FsError::InvalidMetadata)?,
            ),
        };

        // perms will never be more than 3 bytes
        Ok(metadata.permissions.to_le_bytes()[0..3].try_into().unwrap())
    }

    fn list(&mut self, path: &str) -> Result<Vec<String>, FileError> {
        let file_systems = FILESYSTEMS.lock();
        let fs = file_systems
            .get(&(self.bus, self.dsk))
            .ok_or(FileError::NotFoundError(FsError::FilesystemNotFound.into()))?;
        let mut files = Vec::new();
        for inode in fs.phys_fs.inode_table.iter() {
            let file_name = String::from_utf8(inode.file_name.to_vec()).unwrap();
            if file_name.starts_with(path) {
                files.push(file_name);
            }
        }
        Ok(files)
    }
}

/// get the required buffer size for a given file
pub fn get_buffer_size(bus: usize, dsk: usize, path: &str) -> Result<usize, FileError> {
    let mut file_systems = FILESYSTEMS.lock();
    let fs = file_systems
        .get_mut(&(bus, dsk))
        .ok_or(FileError::NotFoundError(FsError::FilesystemNotFound.into()))?;
    let (data, _) = fs.phys_fs.read_file(path)?;
    Ok(data.len())
}

/// get the selected filesystem as a mutable reference
pub fn get_fs_mut(bus: usize, dsk: usize) -> Result<&'static mut VirtFs, FileError> {
    let mut file_systems = FILESYSTEMS.lock();
    let fs = file_systems
        .get_mut(&(bus, dsk))
        .ok_or(FileError::NotFoundError(FsError::FilesystemNotFound.into()))?;
    // Use Box::leak to safely extend the lifetime of the reference to 'static
    Ok(Box::leak(Box::new(fs.clone())))
}

/// get the first good filesystem's bus and device
pub fn get_first_good_fs() -> Result<(usize, usize), FileError> {
    let file_systems = FILESYSTEMS.lock();
    if let Some(((bus, dsk), _)) = file_systems.iter().next() {
        Ok((*bus, *dsk))
    } else {
        Err(FileError::NotFoundError(FsError::FilesystemNotFound.into()))
    }
}

/// load the filesystem from the disk
pub fn load_fs(bus: usize, dsk: usize) -> Result<(), FileError> {
    VirtFs::from_disk(bus, dsk).map_err(|f| f.into())
}

/// add a filesystem to the list of filesystems
pub fn add_fs(bus: usize, dsk: usize, size_of_new: Option<u32>) -> Result<(), FileError> {
    let mut file_systems = FILESYSTEMS.lock();
    if file_systems.contains_key(&(bus, dsk)) {
        return Err(FileError::WriteError(FsError::FilesystemExists.into()));
    }

    if size_of_new.is_some() {
        file_systems.insert((bus, dsk), VirtFs {
            phys_fs: PhysFs {
                superblock: Superblock {
                    magic_number: MAGIC_NUMBER,
                    disk_size: size_of_new.unwrap() as u64,
                    inode_table_size: 1024,
                    data_block_size: 512,
                    num_inodes: 1024,
                    num_data_blocks: (size_of_new.unwrap() as u64 / 512) - 1024 - 1, // superblock + inode table
                },
                inode_table: vec![
                    Inode {
                        num_data_blocks: 0,
                        data_block_pointers: [0; 12],
                        single_indirect_block_pointer: 0,
                        double_indirect_block_pointer: 0,
                        triple_indirect_block_pointer: 0,
                        file_name: [0; 384],
                    };
                    1024
                ],
                data_blocks: vec![DataBlock { data: [0; 512] }; 1024],
            },
            bus,
            dsk,
            open_files: Vec::new(),
        });
        Ok(())
    } else {
        Err(FileError::NotFoundError(FsError::FilesystemNotFound.into()))
    }
}

/// load the filesystem
pub fn init() {
    trace!("Initializing filesystems");

    #[cfg(not(test))]
    // during tests, we don't want to load the filesystem, as we don't currently attach a disk
    {
        let res: Result<(), FileError> = load_fs(0, 1);

        if let Err(err) = res {
            warn!("Failed to load filesystem: {:?}", err);
        }
    }
}

/// test the creation of a file
#[test_case]
fn test_create_file() {
    let mut fs = VirtFs {
        phys_fs: PhysFs {
            superblock: Superblock {
                magic_number: MAGIC_NUMBER,
                disk_size: 1024,
                inode_table_size: 1024,
                data_block_size: 512,
                num_inodes: 1024,
                num_data_blocks: 1024,
            },
            inode_table: vec![
                Inode {
                    num_data_blocks: 0,
                    data_block_pointers: [0; 12],
                    single_indirect_block_pointer: 0,
                    double_indirect_block_pointer: 0,
                    triple_indirect_block_pointer: 0,
                    file_name: [0; 384],
                };
                1024
            ],
            data_blocks: vec![DataBlock { data: [0; 512] }; 1024],
        },
        bus: 0,
        dsk: 0,
        open_files: Vec::new(),
    };

    FILESYSTEMS.lock().insert((0, 0), fs.clone());

    fs.open("test.txt", ALL_FLAGS).unwrap();
    assert!(fs.exists("test.txt"));
}

/// test the writing of a file
#[test_case]
fn test_write_file() {
    let mut fs = VirtFs {
        phys_fs: PhysFs {
            superblock: Superblock {
                magic_number: MAGIC_NUMBER,
                disk_size: 1024,
                inode_table_size: 1024,
                data_block_size: 512,
                num_inodes: 1024,
                num_data_blocks: 1024,
            },
            inode_table: vec![
                Inode {
                    num_data_blocks: 0,
                    data_block_pointers: [0; 12],
                    single_indirect_block_pointer: 0,
                    double_indirect_block_pointer: 0,
                    triple_indirect_block_pointer: 0,
                    file_name: [0; 384],
                };
                1024
            ],
            data_blocks: vec![DataBlock { data: [0; 512] }; 1024],
        },
        bus: 0,
        dsk: 0,
        open_files: Vec::new(),
    };

    fs.phys_fs.create_file("test.txt", [0, 0, 0], 0).unwrap();

    FILESYSTEMS.lock().insert((0, 0), fs.clone());

    let data = b"Hello, world!";
    fs.open("test.txt", FileFlags::Write as u8)
        .expect("Failed to open file")
        .write(data)
        .expect("Failed to write to file");

    let mut buf = [0; 512];
    fs.open("test.txt", FileFlags::Read as u8)
        .expect("Failed to open file")
        .read(&mut buf)
        .expect("Failed to read from file");

    assert_eq!(&buf[..data.len()], data);

    FILESYSTEMS.lock().remove(&(0, 0));
}

/// test chmod
#[test_case]
fn test_chmod_file() {
    let mut fs = VirtFs {
        phys_fs: PhysFs {
            superblock: Superblock {
                magic_number: MAGIC_NUMBER,
                disk_size: 1024,
                inode_table_size: 1024,
                data_block_size: 512,
                num_inodes: 1024,
                num_data_blocks: 1024,
            },
            inode_table: vec![
                Inode {
                    num_data_blocks: 0,
                    data_block_pointers: [0; 12],
                    single_indirect_block_pointer: 0,
                    double_indirect_block_pointer: 0,
                    triple_indirect_block_pointer: 0,
                    file_name: [0; 384],
                };
                1024
            ],
            data_blocks: vec![DataBlock { data: [0; 512] }; 1024],
        },
        bus: 0,
        dsk: 0,
        open_files: Vec::new(),
    };

    FILESYSTEMS.lock().insert((0, 0), fs.clone());

    fs.open("test.txt", ALL_FLAGS).unwrap();

    fs.chmod("test.txt", [1, 1, 1]).unwrap();
    let perms = fs.get_perms("test.txt").unwrap();
    assert_eq!(perms, [1, 1, 1]);
}

/// test chown
#[test_case]
fn test_chown_file() {
    let mut fs = VirtFs {
        phys_fs: PhysFs {
            superblock: Superblock {
                magic_number: MAGIC_NUMBER,
                disk_size: 1024,
                inode_table_size: 1024,
                data_block_size: 512,
                num_inodes: 1024,
                num_data_blocks: 1024,
            },
            inode_table: vec![
                Inode {
                    num_data_blocks: 0,
                    data_block_pointers: [0; 12],
                    single_indirect_block_pointer: 0,
                    double_indirect_block_pointer: 0,
                    triple_indirect_block_pointer: 0,
                    file_name: [0; 384],
                };
                1024
            ],
            data_blocks: vec![DataBlock { data: [0; 512] }; 1024],
        },
        bus: 0,
        dsk: 0,
        open_files: Vec::new(),
    };

    FILESYSTEMS.lock().insert((0, 0), fs.clone());

    fs.open("test.txt", ALL_FLAGS).unwrap();

    fs.chown("test.txt", 1).unwrap();
    let owner = fs.get_owner("test.txt").unwrap();
    assert_eq!(owner, 1);

    FILESYSTEMS.lock().remove(&(0, 0));
}

/// test delete
#[test_case]
fn test_delete_file() {
    let mut fs = VirtFs {
        phys_fs: PhysFs {
            superblock: Superblock {
                magic_number: MAGIC_NUMBER,
                disk_size: 1024,
                inode_table_size: 1024,
                data_block_size: 512,
                num_inodes: 1024,
                num_data_blocks: 1024,
            },
            inode_table: vec![
                Inode {
                    num_data_blocks: 0,
                    data_block_pointers: [0; 12],
                    single_indirect_block_pointer: 0,
                    double_indirect_block_pointer: 0,
                    triple_indirect_block_pointer: 0,
                    file_name: [0; 384],
                };
                1024
            ],
            data_blocks: vec![DataBlock { data: [0; 512] }; 1024],
        },
        bus: 0,
        dsk: 0,
        open_files: Vec::new(),
    };

    FILESYSTEMS.lock().insert((0, 0), fs.clone());

    fs.open("test.txt", ALL_FLAGS).unwrap();

    fs.delete("test.txt").unwrap();
    assert_eq!(fs.exists("test.txt"), false);

    FILESYSTEMS.lock().remove(&(0, 0));
}
