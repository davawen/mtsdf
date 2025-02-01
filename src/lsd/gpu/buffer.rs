use std::marker::PhantomData;

use sdl3_sys::gpu::*;
use bitflags::bitflags;

use crate::error::{ErrorKind, Result};

use super::{VertexBufferBinding, CopyPass, Device, StorageBufferReadBinding, StorageBufferReadWriteBinding};

bitflags! {
    pub struct BufferUsage: u32 {
        const Vertex = SDL_GPU_BUFFERUSAGE_VERTEX;
        const Index = SDL_GPU_BUFFERUSAGE_INDEX;
        const Indirect = SDL_GPU_BUFFERUSAGE_INDIRECT;
        const GraphicsStorageRead = SDL_GPU_BUFFERUSAGE_GRAPHICS_STORAGE_READ;
        const ComputeStorageRead = SDL_GPU_BUFFERUSAGE_COMPUTE_STORAGE_READ;
        const ComputeStorageWrite = SDL_GPU_BUFFERUSAGE_COMPUTE_STORAGE_WRITE;
    }
}


/// A GPU owned memory buffer.
///
/// Can only use [`Copy`]-able types
/// (since they should be copiable to the GPU).
/// 
/// Composite types should be marked as #[repr(C)] or serialized.
/// (You should take a lot of care in aligning data,
/// GLSL often does not have the same layout as C)
///
/// To get data from and to a [`Buffer`],
/// you can use the [`Buffer::fill_from_slice`] function.
/// For more complex data transfers,
/// see [`UploadTransferBuffer`] and [`DownloadTransferBuffer`].
pub struct Buffer<'a, T: Copy> {
    pub ptr: *mut SDL_GPUBuffer,
    device: &'a Device,
    len: usize,
    _data_type: PhantomData<T>
}

impl<'a, T: Copy> Buffer<'a, T> {
    /// Creates a new buffer of given len on the given device.
    /// Note that the `len` parameter refers to the number of elements in the buffer,
    /// and not the size in bytes of the buffer.
    pub fn new(device: &'a Device, len: usize, usage: BufferUsage) -> Result<Self> {
        let mut create_info = SDL_GPUBufferCreateInfo {
            usage: usage.bits(),
            size: (len * std::mem::size_of::<T>()) as u32,
            props: 0
        };

        unsafe { 
            let ptr = SDL_CreateGPUBuffer(device.ptr, &raw mut create_info);
            if ptr.is_null() {
                return Err(ErrorKind::BufferCreation.open())
            }
            Ok(Buffer { ptr, device, len, _data_type: PhantomData })
        }
    }

    /// Upload data to the buffer from an [`UploadTransferBuffer`].
    /// 
    /// # Panics
    /// Panics if the access is out of bounds 
    pub fn fill_from_transfer_buffer(
        &self, 
        copy_pass: &CopyPass,
        transfer_buffer: &UploadTransferBuffer<T>, transfer_offset: usize,
        buffer_offset: usize
    ) {
        if transfer_offset > transfer_buffer.len {
            panic!("out of bounds access to transfer buffer while writing to GPU buffer.\nlen is {}, tried to write from offset {}", 
                transfer_buffer.len, transfer_offset
            );
        }
        if buffer_offset + transfer_buffer.len - transfer_offset > self.len {
            panic!("out of bounds write to GPU buffer (len is {}, tried to write transfer buffer of size {} with offset of {})",
                self.len, transfer_buffer.len, transfer_offset
            );
        }

        let location = SDL_GPUTransferBufferLocation {
            transfer_buffer: transfer_buffer.ptr,
            offset: (transfer_offset * std::mem::size_of::<T>()) as u32
        };

        let destination = SDL_GPUBufferRegion {
            buffer: self.ptr,
            offset: (buffer_offset * std::mem::size_of::<T>()) as u32,
            size: (transfer_buffer.len * std::mem::size_of::<T>()) as u32
        };

        unsafe { 
            SDL_UploadToGPUBuffer(copy_pass.ptr, &location as *const _, &destination as *const _, false);
        }
    }

    /// Fills a GPU buffer from a slice of clonable objects, at the given offset.
    /// This creates a transfer buffer, fills it with the slice,
    /// copies it to the GPU buffer, and destroys it.
    /// Note that if you want to fill multiple buffers, it would be better to create a single transfer buffer.
    ///
    /// # Panics
    /// Panics if the write is out of bounds.
    pub fn fill_from_slice(&self, copy_pass: &CopyPass, offset: usize, data: &[T]) -> Result<()> {
        if data.len() + offset > self.len {
            panic!("out of bounds write to GPU buffer (len is {}, tried to write slice of len {} at offset {})", 
                self.len, data.len(), offset
            );
        }

        let mut transfer_buffer = UploadTransferBuffer::new(self.device, data.len())?;
        transfer_buffer.fill_from_slice(self.device, data, 0)?;
        self.fill_from_transfer_buffer(copy_pass, &transfer_buffer, 0, offset);
        Ok(())
    }

    /// Constructs a vertex buffer binding from the given index.
    /// - `index`: The element index at which the buffer is bound
    /// 
    /// # Panics
    /// Panics if offset is out of bounds
    pub fn vertex_binding(&self, index: usize) -> VertexBufferBinding {
        if index >= self.len { 
            panic!("attempted to bind GPU buffer out of bounds (len is {}, index is {})",
                self.len, index
            );
        }

        VertexBufferBinding {
            inner: SDL_GPUBufferBinding {
                buffer: self.ptr,
                offset: (index * std::mem::size_of::<T>()) as u32
            },
            _lifetime: PhantomData
        }
    }

    /// Creates a read only storage buffer binding from this buffer.
    pub fn read_binding(&self) -> StorageBufferReadBinding {
        StorageBufferReadBinding {
            inner: self.ptr,
            _lifetime: PhantomData
        }
    }

    /// Creates a read write storage buffer binding from this buffer.
    pub fn read_write_binding(&self, cycle: bool) -> StorageBufferReadWriteBinding {
        StorageBufferReadWriteBinding {
            inner: SDL_GPUStorageBufferReadWriteBinding {
                buffer: self.ptr, cycle,
                padding1: 0, padding2: 0, padding3: 0
            },
            _lifetime: PhantomData
        }
    }
}

impl<T: Copy> Drop for Buffer<'_, T> {
    fn drop(&mut self) {
        unsafe { 
            SDL_ReleaseGPUBuffer(self.device.ptr, self.ptr);
        }
    }
}

pub struct UploadTransferBuffer<T: Copy> {
    pub ptr: *mut SDL_GPUTransferBuffer,
    len: usize,
    _data_type: PhantomData<T>
}

pub struct MappedTransferBuffer<'a, T: Copy> {
    device_ptr: *mut SDL_GPUDevice,
    buffer_ptr: *mut SDL_GPUTransferBuffer,
    slice: &'a mut [T]
}

impl<T: Copy> UploadTransferBuffer<T> {
    pub fn new(device: &Device, len: usize) -> Result<Self> {
        let info = SDL_GPUTransferBufferCreateInfo {
            size: (len * std::mem::size_of::<T>()) as u32,
            usage: SDL_GPU_TRANSFERBUFFERUSAGE_UPLOAD,
            props: 0
        };

        unsafe {
            let ptr = SDL_CreateGPUTransferBuffer(device.ptr, &info as *const _);
            if ptr.is_null() {
                return Err(ErrorKind::TransferBufferCreation.open());
            }
            Ok(UploadTransferBuffer { ptr, len, _data_type: PhantomData })
        }
    }

    /// Returns mapped memory you can write to.
    /// - `cycle`: Cycles the buffer if it is already bound/mapped
    pub fn map(&mut self, device: &Device, cycle: bool) -> Result<MappedTransferBuffer<T>> {
        unsafe {
            let ptr = SDL_MapGPUTransferBuffer(device.ptr, self.ptr, cycle);
            if ptr.is_null() {
                return Err(ErrorKind::TransferBufferMap.open());
            }
            let slice = std::slice::from_raw_parts_mut(ptr as *mut T, self.len);
            Ok(MappedTransferBuffer { device_ptr: device.ptr, buffer_ptr: self.ptr, slice })
        }
    }

    /// Fills the transfer buffer with some cloneable data at the given offset.
    /// This handles mapping -> copying -> unmapping the buffer for you.
    /// If you have more complex data patterns, prefer mapping the buffer.
    /// 
    /// # Panics
    /// Panics if the given slice plus offset is out of the buffer bounds
    pub fn fill_from_slice(&mut self, device: &Device, data: &[T], offset: usize) -> Result<()> {
        let mut mapped = self.map(device, false)?;
        mapped.slice_mut()[offset..offset+data.len()].clone_from_slice(data);
        Ok(())
    }

    /// Returns the number of elements of type `T` in the buffer.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Interpret the transfer buffer as having another type,
    /// Changes the length of the buffer to fit the new type.
    /// 
    /// # Panics
    /// Panics if the size of the buffer in bytes is not a multiple of `size_of::<J>()`.
    ///
    /// # Safety
    /// The transmutation should not violate any invariants of the type.
    /// There are probably problems with alignment
    /// I'm not aware of that could cause undefined behaviour.
    pub unsafe fn transmute<J: Copy>(self) -> UploadTransferBuffer<J> {
        let new_size = std::mem::size_of::<J>();
        let old_size = std::mem::size_of::<T>();
        
        let len_bytes = self.len * old_size;
        let len = if len_bytes % new_size == 0 {
            len_bytes / new_size
        } else {
            panic!("attempted to transmute buffer with wrongly aligned size.\nlen is {}, size in bytes is {len_bytes}, new_size is {new_size}, {} bytes are left dangling.",
                self.len, len_bytes % new_size
            );
        };

        UploadTransferBuffer {
            ptr: self.ptr, len, _data_type: PhantomData
        }
    }
}

impl<T: Copy> MappedTransferBuffer<'_, T> {
    pub fn slice(&self) -> &[T] {
        self.slice
    }
    pub fn slice_mut(&mut self) -> &mut [T] {
        self.slice
    }

    /// Calls drop
    pub fn unmap(self) {}
}

impl<T: Copy> std::ops::Index<usize> for MappedTransferBuffer<'_, T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        &self.slice[index]
    }
}

impl<T: Copy> std::ops::IndexMut<usize> for MappedTransferBuffer<'_, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.slice[index]
    }
}

impl<T: Copy> Drop for MappedTransferBuffer<'_, T> {
    fn drop(&mut self) {
        unsafe { SDL_UnmapGPUTransferBuffer(self.device_ptr, self.buffer_ptr) }
    }
}

