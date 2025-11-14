use crate::GpuLayout;
use alloc::vec::Vec;
use core::mem::MaybeUninit;

pub trait WriteInto<Layout> {
    fn write_into<B: BufferMut>(&self, writer: &mut Writer<B>);
}

pub trait ReadFrom<Layout> {
    fn read_from<B: BufferRef>(&mut self, reader: &mut Reader<B>);
}

pub trait CreateFrom<Layout> {
    fn create_from<B: BufferRef>(reader: &mut Reader<B>) -> Self;
}

#[allow(clippy::len_without_is_empty)]
pub trait BufferRef {
    fn len(&self) -> usize;

    fn read<const N: usize>(&self, offset: usize) -> &[u8; N];

    fn read_slice(&self, offset: usize, val: &mut [u8]);
}

pub trait BufferMut {
    fn capacity(&self) -> usize;

    fn write<const N: usize>(&mut self, offset: usize, val: &[u8; N]);

    fn write_slice(&mut self, offset: usize, val: &[u8]);

    #[inline]
    fn try_enlarge(&mut self, wanted: usize) -> Result<(), EnlargeError> {
        if wanted > self.capacity() {
            Err(EnlargeError)
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("could not enlarge buffer")]
pub struct EnlargeError;

impl From<alloc::collections::TryReserveError> for EnlargeError {
    fn from(_: alloc::collections::TryReserveError) -> Self {
        Self
    }
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum AccessError {
    #[error("could not read/write {expected} bytes from/into {found} bytes sized buffer")]
    BufferTooSmall { expected: u64, found: u64 },
}

pub struct Writer<B> {
    cursor: Cursor<B>,
}

impl<B: BufferMut> Writer<B> {
    #[inline]
    pub fn new<Layout, T>(data: &T, buffer: B, offset: usize) -> Result<Self, AccessError>
    where
        T: ?Sized + GpuLayout<Layout>,
    {
        let mut cursor = Cursor::new(buffer, offset);
        let size = data.size().get();
        cursor
            .try_enlarge(offset + size as usize)
            .map_err(|_| AccessError::BufferTooSmall {
                expected: size,
                found: cursor.capacity() as u64,
            })?;
        Ok(Self { cursor })
    }

    #[inline]
    pub fn advance(&mut self, amount: usize) {
        self.cursor.advance(amount);
    }

    #[inline]
    pub fn write<const N: usize>(&mut self, val: &[u8; N]) {
        self.cursor.write(val);
    }

    #[inline]
    pub fn write_slice(&mut self, val: &[u8]) {
        self.cursor.write_slice(val);
    }
}

pub struct Reader<B> {
    cursor: Cursor<B>,
}

impl<B: BufferRef> Reader<B> {
    pub fn new<Layout, T>(buffer: B, offset: usize) -> Result<Self, AccessError>
    where
        T: ?Sized + GpuLayout<Layout>,
    {
        let cursor = Cursor::new(buffer, offset);
        if cursor.remaining() < T::METADATA.min_size().get() as usize {
            Err(AccessError::BufferTooSmall {
                expected: T::METADATA.min_size().get(),
                found: cursor.remaining() as u64,
            })
        } else {
            Ok(Self { cursor })
        }
    }

    #[inline]
    pub fn advance(&mut self, amount: usize) {
        self.cursor.advance(amount);
    }

    #[inline]
    pub fn remaining(&self) -> usize {
        self.cursor.remaining()
    }

    #[inline]
    pub fn read<const N: usize>(&mut self) -> &[u8; N] {
        self.cursor.read()
    }

    #[inline]
    pub fn read_slice(&mut self, val: &mut [u8]) {
        self.cursor.read_slice(val)
    }
}

struct Cursor<B> {
    buffer: B,
    pos: usize,
}

impl<B> Cursor<B> {
    #[inline]
    fn new(buffer: B, offset: usize) -> Self {
        Self {
            buffer,
            pos: offset,
        }
    }

    #[inline]
    fn advance(&mut self, amount: usize) {
        self.pos += amount;
    }
}

impl<B: BufferRef> Cursor<B> {
    #[inline]
    fn remaining(&self) -> usize {
        self.buffer.len().saturating_sub(self.pos)
    }

    #[inline]
    fn read<const N: usize>(&mut self) -> &[u8; N] {
        let res = self.buffer.read(self.pos);
        self.pos += N;
        res
    }

    #[inline]
    fn read_slice(&mut self, val: &mut [u8]) {
        self.buffer.read_slice(self.pos, val);
        self.pos += val.len();
    }
}

impl<B: BufferMut> Cursor<B> {
    #[inline]
    fn capacity(&self) -> usize {
        self.buffer.capacity().saturating_sub(self.pos)
    }

    #[inline]
    fn write<const N: usize>(&mut self, val: &[u8; N]) {
        self.buffer.write(self.pos, val);
        self.pos += N;
    }

    #[inline]
    fn write_slice(&mut self, val: &[u8]) {
        self.buffer.write_slice(self.pos, val);
        self.pos += val.len();
    }

    #[inline]
    fn try_enlarge(&mut self, wanted: usize) -> Result<(), EnlargeError> {
        self.buffer.try_enlarge(wanted)
    }
}

impl BufferRef for [u8] {
    #[inline]
    fn len(&self) -> usize {
        self.len()
    }

    #[inline]
    fn read<const N: usize>(&self, offset: usize) -> &[u8; N] {
        let src = &self[offset..offset + N];

        // SAFETY
        // casting to &[T; N] is safe since src is a &[T] of length N
        unsafe { &*(src.as_ptr() as *const [u8; N]) }
    }

    #[inline]
    fn read_slice(&self, offset: usize, val: &mut [u8]) {
        val.copy_from_slice(&self[offset..offset + val.len()])
    }
}

impl<const LEN: usize> BufferRef for [u8; LEN] {
    #[inline]
    fn len(&self) -> usize {
        <[u8] as BufferRef>::len(self)
    }

    #[inline]
    fn read<const N: usize>(&self, offset: usize) -> &[u8; N] {
        <[u8] as BufferRef>::read(self, offset)
    }

    #[inline]
    fn read_slice(&self, offset: usize, val: &mut [u8]) {
        <[u8] as BufferRef>::read_slice(self, offset, val)
    }
}

impl BufferRef for Vec<u8> {
    #[inline]
    fn len(&self) -> usize {
        <[u8] as BufferRef>::len(self)
    }

    #[inline]
    fn read<const N: usize>(&self, offset: usize) -> &[u8; N] {
        <[u8] as BufferRef>::read(self, offset)
    }

    #[inline]
    fn read_slice(&self, offset: usize, val: &mut [u8]) {
        <[u8] as BufferRef>::read_slice(self, offset, val)
    }
}

impl BufferMut for [u8] {
    #[inline]
    fn capacity(&self) -> usize {
        self.len()
    }

    #[inline]
    fn write<const N: usize>(&mut self, offset: usize, val: &[u8; N]) {
        let src = &mut self[offset..offset + N];

        // SAFETY
        // casting to &mut [T; N] is safe since src is a &mut [T] of length N
        *unsafe { &mut *(src.as_mut_ptr() as *mut [u8; N]) } = *val;
    }

    #[inline]
    fn write_slice(&mut self, offset: usize, val: &[u8]) {
        self[offset..offset + val.len()].copy_from_slice(val);
    }
}

impl BufferMut for [MaybeUninit<u8>] {
    #[inline]
    fn capacity(&self) -> usize {
        self.len()
    }

    #[inline]
    fn write<const N: usize>(&mut self, offset: usize, val: &[u8; N]) {
        // SAFETY: &[u8; N] and &[MaybeUninit<u8>; N] have the same layout
        let val: &[MaybeUninit<u8>; N] = unsafe { core::mem::transmute(val) };
        let src = &mut self[offset..offset + N];

        // SAFETY
        // casting to &mut [T; N] is safe since src is a &mut [T] of length N
        *unsafe { &mut *(src.as_mut_ptr() as *mut [MaybeUninit<u8>; N]) } = *val;
    }

    #[inline]
    fn write_slice(&mut self, offset: usize, val: &[u8]) {
        // SAFETY: &[u8] and &[MaybeUninit<u8>] have the same layout
        let val: &[MaybeUninit<u8>] = unsafe { core::mem::transmute(val) };
        self[offset..offset + val.len()].copy_from_slice(val);
    }
}

impl<const LEN: usize> BufferMut for [u8; LEN] {
    #[inline]
    fn capacity(&self) -> usize {
        <[u8] as BufferMut>::capacity(self)
    }

    #[inline]
    fn write<const N: usize>(&mut self, offset: usize, val: &[u8; N]) {
        <[u8] as BufferMut>::write(self, offset, val)
    }

    #[inline]
    fn write_slice(&mut self, offset: usize, val: &[u8]) {
        <[u8] as BufferMut>::write_slice(self, offset, val);
    }
}

impl<const LEN: usize> BufferMut for [MaybeUninit<u8>; LEN] {
    #[inline]
    fn capacity(&self) -> usize {
        <[MaybeUninit<u8>] as BufferMut>::capacity(self)
    }

    #[inline]
    fn write<const N: usize>(&mut self, offset: usize, val: &[u8; N]) {
        <[MaybeUninit<u8>] as BufferMut>::write(self, offset, val)
    }

    #[inline]
    fn write_slice(&mut self, offset: usize, val: &[u8]) {
        <[MaybeUninit<u8>] as BufferMut>::write_slice(self, offset, val);
    }
}

impl BufferMut for Vec<u8> {
    #[inline]
    fn capacity(&self) -> usize {
        self.capacity()
    }

    #[inline]
    fn write<const N: usize>(&mut self, offset: usize, val: &[u8; N]) {
        <[u8] as BufferMut>::write(self, offset, val)
    }

    #[inline]
    fn write_slice(&mut self, offset: usize, val: &[u8]) {
        <[u8] as BufferMut>::write_slice(self, offset, val);
    }

    #[inline]
    fn try_enlarge(&mut self, wanted: usize) -> Result<(), EnlargeError> {
        let additional = wanted.saturating_sub(self.len());
        if additional > 0 {
            self.try_reserve(additional)?;

            let end = unsafe { self.as_mut_ptr().add(self.len()) };
            // SAFETY
            // 1. dst ptr is valid for writes of count * size_of::<T>() bytes since the call to Vec::reserve() succeeded
            // 2. dst ptr is properly aligned since we got it via Vec::as_mut_ptr_range()
            unsafe { end.write_bytes(u8::MIN, additional) }
            // SAFETY
            // 1. new_len is less than or equal to Vec::capacity() since we reserved at least `additional` elements
            // 2. The elements at old_len..new_len are initialized since we wrote `additional` bytes
            unsafe { self.set_len(wanted) }
        }
        Ok(())
    }
}

impl BufferMut for Vec<MaybeUninit<u8>> {
    #[inline]
    fn capacity(&self) -> usize {
        self.capacity()
    }

    #[inline]
    fn write<const N: usize>(&mut self, offset: usize, val: &[u8; N]) {
        <[MaybeUninit<u8>] as BufferMut>::write(self, offset, val)
    }

    #[inline]
    fn write_slice(&mut self, offset: usize, val: &[u8]) {
        <[MaybeUninit<u8>] as BufferMut>::write_slice(self, offset, val)
    }

    #[inline]
    fn try_enlarge(&mut self, wanted: usize) -> Result<(), EnlargeError> {
        let additional = wanted.saturating_sub(self.len());
        if additional > 0 {
            self.try_reserve(additional)?;

            // It's OK to not initialize the extended elements as MaybeUninit allows
            // uninitialized memory.

            // SAFETY
            // 1. new_len is less than or equal to Vec::capacity() since we reserved at least `additional` elements
            // 2. The elements at old_len..new_len are initialized since we wrote `additional` bytes
            // 3. MaybeUninit
            unsafe { self.set_len(wanted) }
        }
        Ok(())
    }
}

macro_rules! impl_buffer_ref_for_wrappers {
    ($($type:ty),*) => {$(
		impl<T: ?Sized + BufferRef> BufferRef for $type {
			#[inline]
            fn len(&self) -> usize {
                T::len(self)
            }

            #[inline]
            fn read<const N: usize>(&self, offset: usize) -> &[u8; N] {
                T::read(self, offset)
            }

            #[inline]
            fn read_slice(&self, offset: usize, val: &mut [u8]) {
                T::read_slice(self, offset, val)
            }
		}
	)*};
}

impl_buffer_ref_for_wrappers!(
    &T,
    &mut T,
    alloc::boxed::Box<T>,
    alloc::rc::Rc<T>,
    alloc::sync::Arc<T>
);

macro_rules! impl_buffer_mut_for_wrappers {
    ($($type:ty),*) => {$(
        impl<T: ?Sized + BufferMut> BufferMut for $type {
            #[inline]
            fn capacity(&self) -> usize {
                T::capacity(self)
            }

            #[inline]
            fn write<const N: usize>(&mut self, offset: usize, val: &[u8; N]) {
                T::write(self, offset, val)
            }

            #[inline]
            fn write_slice(&mut self, offset: usize, val: &[u8]) {
                T::write_slice(self, offset, val)
            }

            #[inline]
            fn try_enlarge(&mut self, wanted: usize) -> core::result::Result<(), EnlargeError> {
                T::try_enlarge(self, wanted)
            }
        }
    )*};
}

impl_buffer_mut_for_wrappers!(&mut T, alloc::boxed::Box<T>);
