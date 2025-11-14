#![no_std]

extern crate alloc;
extern crate self as gpu_layout;
#[cfg(feature = "std")]
extern crate std;

mod common;
mod rw;
mod std140;
mod std430;

pub use gpu_layout_macros::GpuLayout;
pub use rw::{BufferMut, BufferRef, CreateFrom, ReadFrom, Reader, WriteInto, Writer};
pub use std140::Std140;
pub use std430::Std430;

use core::num::NonZeroU64;

pub trait GpuLayout<Layout> {
    type ExtraMetadata;
    const METADATA: Metadata<Self::ExtraMetadata>;

    #[inline]
    fn size(&self) -> NonZeroU64 {
        Self::METADATA.min_size().0
    }
}

pub trait GpuLayoutSize<Layout>: GpuLayout<Layout> {
    const SIZE: NonZeroU64 = Self::METADATA.min_size().0;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlignmentValue(NonZeroU64);

impl AlignmentValue {
    #[inline]
    pub const fn new(val: u64) -> Self {
        if !val.is_power_of_two() {
            panic!("alignment must be power of two");
        }

        // SAFETY: 0 can't be power of two
        unsafe { Self(NonZeroU64::new_unchecked(val)) }
    }

    #[inline]
    pub const fn from_next_power_of_two_size(size: SizeValue) -> Self {
        match size.get().checked_next_power_of_two() {
            None => panic!("Overflow occurred while getting the next power of 2"),
            Some(val) => {
                // SAFETY: This is safe since we got the next_power_of_two
                Self(unsafe { NonZeroU64::new_unchecked(val) })
            }
        }
    }

    #[inline]
    pub const fn get(&self) -> u64 {
        self.0.get()
    }

    #[inline]
    pub const fn padding_needed_for(&self, n: u64) -> u64 {
        let r = n % self.get();
        if r > 0 { self.get() - r } else { 0 }
    }

    /// Will round up the given `n` so that the returned value will be a multiple of this alignment
    #[inline]
    pub const fn round_up(&self, n: u64) -> u64 {
        n + self.padding_needed_for(n)
    }

    /// Will round up the given `n` so that the returned value will be a multiple of this alignment
    #[inline]
    pub const fn round_up_size(&self, n: SizeValue) -> SizeValue {
        SizeValue::new(self.round_up(n.get()))
    }

    #[inline]
    pub const fn max<const N: usize>(input: [AlignmentValue; N]) -> Self {
        let mut max = input[0];
        let mut i = 1;

        while i < N {
            if input[i].get() > max.get() {
                max = input[i];
            }
            i += 1;
        }

        max
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SizeValue(pub NonZeroU64);

impl SizeValue {
    #[inline]
    pub const fn new(val: u64) -> Self {
        match val {
            0 => panic!("Size can't be 0"),
            val => {
                // SAFETY: we checked it's not zero
                Self(unsafe { NonZeroU64::new_unchecked(val) })
            }
        }
    }

    #[inline]
    pub const fn from(val: NonZeroU64) -> Self {
        Self(val)
    }

    #[inline]
    pub const fn get(&self) -> u64 {
        self.0.get()
    }

    #[inline]
    pub const fn mul(self, rhs: u64) -> Self {
        match self.get().checked_mul(rhs) {
            None => panic!("Overflow occurred while multiplying size values!"),
            Some(val) => {
                // SAFETY: This is safe since we checked for overflow
                Self(unsafe { NonZeroU64::new_unchecked(val) })
            }
        }
    }
}

#[derive(Debug)]
pub struct Metadata<E> {
    pub alignment: AlignmentValue,
    pub min_size: SizeValue,
    pub is_pod: bool,
    pub extra: E,
}

impl Metadata<()> {
    #[inline]
    pub const fn from_alignment_and_size(alignment: u64, size: u64) -> Self {
        Self {
            alignment: AlignmentValue::new(alignment),
            min_size: SizeValue::new(size),
            is_pod: false,
            extra: (),
        }
    }
}

// using forget() avoids "destructors cannot be evaluated at compile-time" error
// track #![feature(const_precise_live_drops)] (https://github.com/rust-lang/rust/issues/73255)
impl<E> Metadata<E> {
    #[inline]
    pub const fn alignment(self) -> AlignmentValue {
        let value = self.alignment;
        core::mem::forget(self);
        value
    }

    #[inline]
    pub const fn min_size(self) -> SizeValue {
        let value = self.min_size;
        core::mem::forget(self);
        value
    }

    #[allow(clippy::wrong_self_convention)]
    #[inline]
    pub const fn is_pod(self) -> bool {
        let value = self.is_pod;
        core::mem::forget(self);
        value
    }

    #[inline]
    pub const fn pod(mut self) -> Self {
        self.is_pod = true;
        self
    }

    #[inline]
    pub const fn no_pod(mut self) -> Self {
        self.is_pod = true;
        self
    }
}

#[derive(Debug)]
pub struct ArrayMetadata {
    pub stride: SizeValue,
    pub el_padding: u64,
}

impl Metadata<ArrayMetadata> {
    #[inline]
    pub const fn stride(self) -> SizeValue {
        self.extra.stride
    }

    #[inline]
    pub const fn el_padding(self) -> u64 {
        self.extra.el_padding
    }
}

#[derive(Debug)]
pub struct StructMetadata<const N: usize> {
    pub offsets: [u64; N],
    pub paddings: [u64; N],
}

impl<const N: usize> Metadata<StructMetadata<N>> {
    #[inline]
    pub const fn offset(self, i: usize) -> u64 {
        self.extra.offsets[i]
    }

    #[inline]
    pub const fn last_offset(self) -> u64 {
        self.extra.offsets[N - 1]
    }

    #[inline]
    pub const fn padding(self, i: usize) -> u64 {
        self.extra.paddings[i]
    }
}

#[inline]
pub const fn std140_size_of<T: GpuLayoutSize<Std140>>() -> u64 {
    T::SIZE.get()
}

#[inline]
pub const fn std140_align_of<T: GpuLayout<Std140>>() -> u64 {
    T::METADATA.alignment().get()
}

macro_rules! if_pod_and_little_endian {
    (if pod_and_little_endian($layout:ident) $true:block else $false:block) => {
        #[cfg(target_endian = "little")]
        if <Self as $crate::GpuLayout<$layout>>::METADATA.is_pod() {
            $true
        } else {
            $false
        }
        #[cfg(not(target_endian = "little"))]
        {
            $false
        }
    };
}

use if_pod_and_little_endian;

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    #[test]
    fn derive_check() {
        #[derive(Debug, GpuLayout, PartialEq)]
        struct Test {
            a: u32,
            ar: [f32; 4],
        }

        let test = Test { a: 0, ar: [0.0; 4] };

        let mut buf = Vec::<u8>::new();
        let mut writer = Writer::new::<Std140, _>(&test, &mut buf, 0).unwrap();
        WriteInto::write_into(&test, &mut writer);

        let mut reader = Reader::new::<Std140, Test>(&buf, 0).unwrap();
        let mut anot = Test { a: 1, ar: [0.2; 4] };
        ReadFrom::read_from(&mut anot, &mut reader);

        assert_eq!(test, anot);
    }
}
