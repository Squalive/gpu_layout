use crate::{ArrayMetadata, CreateFrom, GpuLayout, WriteInto};
use crate::{ReadFrom, if_pod_and_little_endian};
use core::mem::MaybeUninit;

impl<Layout, T, const N: usize> WriteInto<Layout> for [T; N]
where
    T: WriteInto<Layout>,
    Self: GpuLayout<Layout, ExtraMetadata = ArrayMetadata>,
{
    #[inline]
    fn write_into<B: crate::BufferMut>(&self, writer: &mut crate::Writer<B>) {
        if_pod_and_little_endian! {
            if pod_and_little_endian(Layout) {
                let ptr = self.as_ptr() as *const u8;
                let byte_slice: &[u8] = unsafe { core::slice::from_raw_parts(ptr, size_of::<Self>()) };
                writer.write_slice(byte_slice);
            } else {
                for elem in self {
                    WriteInto::write_into(elem, writer);
                    writer.advance(Self::METADATA.el_padding() as usize);
                }
            }
        }
    }
}

impl<Layout, T, const N: usize> ReadFrom<Layout> for [T; N]
where
    T: ReadFrom<Layout>,
    Self: GpuLayout<Layout, ExtraMetadata = ArrayMetadata>,
{
    #[inline]
    fn read_from<B: crate::BufferRef>(&mut self, reader: &mut crate::Reader<B>) {
        if_pod_and_little_endian! {
            if pod_and_little_endian(Layout) {
                let ptr = self.as_mut_ptr() as *mut u8;
                let byte_slice: &mut [u8] = unsafe { core::slice::from_raw_parts_mut(ptr, size_of::<Self>()) };
                reader.read_slice(byte_slice);
            } else {
                for elem in self {
                    ReadFrom::read_from(elem, reader);
                    reader.advance(Self::METADATA.el_padding() as usize);
                }
            }
        }
    }
}

impl<Layout, T, const N: usize> CreateFrom<Layout> for [T; N]
where
    T: CreateFrom<Layout>,
    Self: GpuLayout<Layout, ExtraMetadata = ArrayMetadata>,
{
    #[inline]
    fn create_from<B: crate::BufferRef>(reader: &mut crate::Reader<B>) -> Self {
        if_pod_and_little_endian! {
            if pod_and_little_endian(Layout) {
                let mut me = MaybeUninit::zeroed();
                let ptr: *mut MaybeUninit<Self> = &mut me;
                let ptr = ptr.cast::<u8>();
                let byte_slice: &mut [u8] = unsafe { core::slice::from_raw_parts_mut(ptr, size_of::<Self>()) };
                reader.read_slice(byte_slice);
                // SAFETY: All values were properly initialized by reading the bytes.
                unsafe { me.assume_init() }
            } else {
                core::array::from_fn(|_| {
                    let res = CreateFrom::create_from(reader);
                    reader.advance(Self::METADATA.el_padding() as usize);
                    res
                })
            }
        }
    }
}
