/*
 * rustnix-fs
 * at the start of the disk, there is a superblock, which contains the following information:
 * - the size of the disk
 * - the size of the inode table
 * - the size of the data blocks
 * - the number of inodes
 * - the number of data blocks
 * - the number of inodes per block
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

use core::num;

use crate::{
    ata::{BLOCK_SIZE, read, write},
    clk,
    file::File,
    println,
};
use alloc::{vec, vec::Vec};

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct Superblock {
    magic_number: u64,
    disk_size: u64,
    inode_table_size: u64,
    data_block_size: u64,
    num_inodes: u64,
    num_data_blocks: u64,
    inodes_per_block: u64,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct Inode {
    num_data_blocks: u64,
    data_block_pointers: [u64; 12],
    // points to a block that contains pointers to data blocks
    single_indirect_block_pointer: u64,
    double_indirect_block_pointer: u64,
    triple_indirect_block_pointer: u64,
    file_name: [u8; 384],
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DataBlock {
    pub data: [u8; 512],
}

#[derive(Debug)]
#[repr(C)]
pub struct FileMetadata {
    owner: u64,
    creation_time: u64,
    modification_time: u64,
    access_time: u64,
    permissions: u64, // Unix-style
}

pub struct PhysFs {
    superblock: Superblock,
    inode_table: Vec<Inode>,
    pub data_blocks: Vec<DataBlock>,
}

fn read_sector(bus: u8, device: u8, sector: u32) -> Result<Vec<u8>, ()> {
    let mut buf = vec![0; BLOCK_SIZE];
    read(bus, device, sector, &mut buf)?;
    Ok(buf)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsError {
    InvalidPath,
    FileNotFound,
    FileExists,
    DiskFull,
    OutOfInodes,
    OutOfDataBlocks,
    InvalidInode,
    InvalidDataBlock,
    InvalidSuperblock,
    InvalidInodeTable,
    InvalidMetadata,
    WriteError,
    ReadError,
}

impl PhysFs {
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
            inodes_per_block: u64::from_le_bytes(
                sector_data[48..56]
                    .try_into()
                    .map_err(|_| FsError::InvalidSuperblock)?,
            ),
        };

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

    pub fn write_to_disk(&self, bus: usize, device: usize) -> Result<(), FsError> {
        // write the superblock to the disk
        let mut sector_data = vec![0; BLOCK_SIZE];
        sector_data[0..8].copy_from_slice(&self.superblock.magic_number.to_le_bytes());
        sector_data[8..16].copy_from_slice(&self.superblock.disk_size.to_le_bytes());
        sector_data[16..24].copy_from_slice(&self.superblock.inode_table_size.to_le_bytes());
        sector_data[24..32].copy_from_slice(&self.superblock.data_block_size.to_le_bytes());
        sector_data[32..40].copy_from_slice(&self.superblock.num_inodes.to_le_bytes());
        sector_data[40..48].copy_from_slice(&self.superblock.num_data_blocks.to_le_bytes());
        sector_data[48..56].copy_from_slice(&self.superblock.inodes_per_block.to_le_bytes());
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

    fn find_empty_data_block(&self) -> Result<u64, FsError> {
        for i in 1..self.superblock.num_data_blocks {
            if self.data_blocks[i as usize].data == [0; 512] {
                return Ok(i);
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

    pub fn create_file(
        &mut self,
        file_name: &str,
        perms: [u8; 3],
        owner: u64,
    ) -> Result<(), FsError> {
        let mut inode = Inode {
            num_data_blocks: 1,
            data_block_pointers: [0; 12],
            single_indirect_block_pointer: 0,
            double_indirect_block_pointer: 0,
            triple_indirect_block_pointer: 0,
            file_name: [0; 384],
        };

        let data_block = self.find_empty_data_block()?;
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

    pub fn read_file(&self, file_name: &str) -> Result<(Vec<u8>, FileMetadata), FsError> {
        let inode = self.find_inode_by_name(file_name)?;

        println!(
            "inode.data_block_pointers[0]: {:?}",
            inode.data_block_pointers
        );

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

        let mut data = Vec::new();
        for i in 1..inode.num_data_blocks {
            data.extend_from_slice(
                &self.data_blocks[inode.data_block_pointers[i as usize] as usize].data,
            );
        }

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

    pub fn write_file(
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
        let mut data_blocks_pointers = vec![0u64; (num_data_blocks + 1) as usize];

        data_blocks_pointers[0] = inode.data_block_pointers[0];

        // Use existing data blocks first
        let mut existing_blocks_used = 0;
        for i in 1..inode.num_data_blocks.min(num_data_blocks) {
            data_blocks_pointers[i as usize] = inode.data_block_pointers[i as usize];
            self.write_to_data_block(
                inode.data_block_pointers[i as usize],
                &data[i as usize * 512..(i + 1) as usize * 512],
            )?;
            existing_blocks_used += 1;
        }

        // Allocate new data blocks if needed
        for i in existing_blocks_used..num_data_blocks {
            let data_block = self.find_empty_data_block()?;
            data_blocks_pointers[i as usize] = data_block;

            self.write_to_data_block(data_block, &data[i as usize * 512..(i + 1) as usize * 512])?;
        }

        // Update inode with new data block pointers
        let mut updated_inode = inode.clone();
        updated_inode.num_data_blocks = num_data_blocks + 1; // metadata block
        updated_inode.data_block_pointers[1..(num_data_blocks + 1) as usize]
            .copy_from_slice(&data_blocks_pointers[..num_data_blocks as usize]);

        self.update_inode(updated_inode)?;

        Ok(())
    }
}

pub fn create_dummy_fs() -> PhysFs {
    let superblock = Superblock {
        magic_number: 0xdeadbeef,
        disk_size: 1024 * 1024, // 1 MB
        inode_table_size: 256,
        data_block_size: 512,
        num_inodes: 1024,
        num_data_blocks: 1024,
        inodes_per_block: 4,
    };

    let inode_table = vec![
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

    let data_blocks = vec![DataBlock { data: [0; 512] }; superblock.num_data_blocks as usize];

    PhysFs {
        superblock,
        inode_table,
        data_blocks,
    }
}
