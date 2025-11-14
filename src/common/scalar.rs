use core::{
    num::{NonZeroI32, NonZeroU32, Wrapping},
    sync::atomic::{AtomicI32, AtomicU32},
};

macro_rules! impl_traits {
    ($type:ty) => {
        impl_traits!(__helper, $type, );
    };
    ($type:ty, pod) => {
        impl_traits!(__helper, $type, .pod());
    };
    (__helper, $type:ty, $($tail:tt)*) => {
        impl<Layout> $crate::GpuLayout<Layout> for $type {
            type ExtraMetadata = ();
            const METADATA: $crate::Metadata<Self::ExtraMetadata> = $crate::Metadata::from_alignment_and_size(4, 4) $($tail)*;
        }

        impl<Layout> $crate::GpuLayoutSize<Layout> for $type {}
    };
}

macro_rules! impl_for_pod {
    ($type:ty) => {
        impl_traits!($type, pod);

        impl<Layout> $crate::WriteInto<Layout> for $type {
            #[inline]
            fn write_into<B: $crate::BufferMut>(&self, writer: &mut $crate::Writer<B>) {
                writer.write(&<$type>::to_le_bytes(*self));
            }
        }

        impl<Layout> $crate::ReadFrom<Layout> for $type {
            #[inline]
            fn read_from<B: $crate::BufferRef>(&mut self, reader: &mut $crate::Reader<B>) {
                *self = <$type>::from_le_bytes(*reader.read());
            }
        }

        impl<Layout> $crate::CreateFrom<Layout> for $type {
            #[inline]
            fn create_from<B: $crate::BufferRef>(reader: &mut $crate::Reader<B>) -> Self {
                <$type>::from_le_bytes(*reader.read())
            }
        }
    };
}

impl_for_pod!(f32);
impl_for_pod!(u32);
impl_for_pod!(i32);

macro_rules! impl_for_non_zero_option {
    ($type:ty) => {
        impl_traits!(Option<$type>);

        impl<Layout> $crate::WriteInto<Layout> for Option<$type> {
            #[inline]
            fn write_into<B: $crate::BufferMut>(&self, writer: &mut $crate::Writer<B>) {
                let value = self.map(|num| num.get()).unwrap_or(0);
                $crate::WriteInto::<Layout>::write_into(&value, writer);
            }
        }

        impl<Layout> $crate::ReadFrom<Layout> for Option<$type> {
            #[inline]
            fn read_from<B: $crate::BufferRef>(&mut self, reader: &mut $crate::Reader<B>) {
                *self = <$type>::new($crate::CreateFrom::<Layout>::create_from(reader));
            }
        }

        impl<Layout> $crate::CreateFrom<Layout> for Option<$type> {
            #[inline]
            fn create_from<B: $crate::BufferRef>(reader: &mut $crate::Reader<B>) -> Self {
                <$type>::new($crate::CreateFrom::<Layout>::create_from(reader))
            }
        }
    };
}

impl_for_non_zero_option!(NonZeroU32);
impl_for_non_zero_option!(NonZeroI32);

macro_rules! impl_for_wrapper {
    ($type:ty) => {
        impl_traits!($type);

        impl<Layout> $crate::WriteInto<Layout> for $type {
            #[inline]
            fn write_into<B: $crate::BufferMut>(&self, writer: &mut $crate::Writer<B>) {
                $crate::WriteInto::<Layout>::write_into(&self.0, writer);
            }
        }

        impl<Layout> $crate::ReadFrom<Layout> for $type {
            #[inline]
            fn read_from<B: $crate::BufferRef>(&mut self, reader: &mut $crate::Reader<B>) {
                $crate::ReadFrom::<Layout>::read_from(&mut self.0, reader);
            }
        }

        impl<Layout> $crate::CreateFrom<Layout> for $type {
            #[inline]
            fn create_from<B: $crate::BufferRef>(reader: &mut $crate::Reader<B>) -> Self {
                Wrapping($crate::CreateFrom::<Layout>::create_from(reader))
            }
        }
    };
}

impl_for_wrapper!(Wrapping<u32>);
impl_for_wrapper!(Wrapping<i32>);

macro_rules! impl_for_atomic {
    ($type:ty) => {
        impl_traits!($type);

        impl<Layout> $crate::WriteInto<Layout> for $type {
            #[inline]
            fn write_into<B: $crate::BufferMut>(&self, writer: &mut $crate::Writer<B>) {
                let value = self.load(::core::sync::atomic::Ordering::Relaxed);
                $crate::WriteInto::<Layout>::write_into(&value, writer);
            }
        }

        impl<Layout> $crate::ReadFrom<Layout> for $type {
            #[inline]
            fn read_from<B: $crate::BufferRef>(&mut self, reader: &mut $crate::Reader<B>) {
                $crate::ReadFrom::<Layout>::read_from(self.get_mut(), reader);
            }
        }

        impl<Layout> $crate::CreateFrom<Layout> for $type {
            fn create_from<B: $crate::BufferRef>(reader: &mut $crate::Reader<B>) -> Self {
                <$type>::new($crate::CreateFrom::<Layout>::create_from(reader))
            }
        }
    };
}

impl_for_atomic!(AtomicU32);
impl_for_atomic!(AtomicI32);
