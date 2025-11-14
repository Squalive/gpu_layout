macro_rules! impl_vector {
    ($n:literal, $type:ty, $el_ty:ty) => {
        const _: () = assert!(
            2 <= $n && $n <= 4,
            "Vector should have at least 2 elements and at most 4"
        );

        impl $crate::GpuLayout<$crate::Std140> for $type
        where
            $el_ty: $crate::GpuLayout<$crate::Std140>,
        {
            type ExtraMetadata = ();
            const METADATA: $crate::Metadata<Self::ExtraMetadata> = {
                let size = $crate::SizeValue::from(
                    <$el_ty as $crate::GpuLayout<$crate::Std140>>::METADATA
                        .min_size()
                        .0,
                )
                .mul($n);
                let alignment = $crate::AlignmentValue::from_next_power_of_two_size(size);

                $crate::Metadata {
                    alignment,
                    min_size: size,
                    is_pod: alignment.padding_needed_for(size.get()) == 0,
                    extra: (),
                }
            };
        }

        impl $crate::GpuLayoutSize<$crate::Std140> for $type where
            $el_ty: $crate::GpuLayoutSize<$crate::Std140>
        {
        }

        impl $crate::WriteInto<$crate::Std140> for $type {
            #[inline]
            fn write_into<B: $crate::BufferMut>(&self, writer: &mut $crate::Writer<B>) {
                let ptr = self.as_ref().as_ptr() as *const u8;
                let byte_slice: &[u8] =
                    unsafe { core::slice::from_raw_parts(ptr, size_of::<Self>()) };
                writer.write_slice(byte_slice);
            }
        }

        impl $crate::ReadFrom<$crate::Std140> for $type {
            #[inline]
            fn read_from<B: $crate::BufferRef>(&mut self, reader: &mut $crate::Reader<B>) {
                let ptr = self.as_mut().as_ptr() as *mut u8;
                let byte_slice: &mut [u8] =
                    unsafe { core::slice::from_raw_parts_mut(ptr, size_of::<Self>()) };
                reader.read_slice(byte_slice);
            }
        }

        impl $crate::CreateFrom<$crate::Std140> for $type {
            #[inline]
            fn create_from<B: $crate::BufferRef>(reader: &mut $crate::Reader<B>) -> Self {
                let mut data = <$type>::ZERO;
                $crate::ReadFrom::<$crate::Std140>::read_from(&mut data, reader);
                data
            }
        }
    };
}

cfg_if::cfg_if! {
    if #[cfg(feature = "glam")] {
        impl_vector!(2, glam::Vec2, f32);
        impl_vector!(2, glam::UVec2, u32);
        impl_vector!(2, glam::IVec2, i32);

        impl_vector!(3, glam::Vec3, f32);
        impl_vector!(3, glam::UVec3, u32);
        impl_vector!(3, glam::IVec3, i32);

        impl_vector!(4, glam::Vec4, f32);
        impl_vector!(4, glam::UVec4, u32);
        impl_vector!(4, glam::IVec4, i32);
    }
}

#[cfg(all(test, feature = "glam"))]
mod glam_tests {
    use crate::{GpuLayout, Std140};

    #[test]
    fn check_vec2_layout() {
        let metadata = <glam::Vec2 as GpuLayout<Std140>>::METADATA;
        assert_eq!(metadata.alignment.get(), 8);
        assert_eq!(metadata.min_size.get(), 8);

        let metadata = <glam::UVec2 as GpuLayout<Std140>>::METADATA;
        assert_eq!(metadata.alignment.get(), 8);
        assert_eq!(metadata.min_size.get(), 8);

        let metadata = <glam::IVec2 as GpuLayout<Std140>>::METADATA;
        assert_eq!(metadata.alignment.get(), 8);
        assert_eq!(metadata.min_size.get(), 8);
    }

    #[test]
    fn check_vec3_layout() {
        let metadata = <glam::Vec3 as GpuLayout<Std140>>::METADATA;
        assert_eq!(metadata.alignment.get(), 16);
        assert_eq!(metadata.min_size.get(), 12);

        let metadata = <glam::UVec3 as GpuLayout<Std140>>::METADATA;
        assert_eq!(metadata.alignment.get(), 16);
        assert_eq!(metadata.min_size.get(), 12);

        let metadata = <glam::IVec3 as GpuLayout<Std140>>::METADATA;
        assert_eq!(metadata.alignment.get(), 16);
        assert_eq!(metadata.min_size.get(), 12);
    }

    #[test]
    fn check_vec4_layout() {
        let metadata = <glam::Vec4 as GpuLayout<Std140>>::METADATA;
        assert_eq!(metadata.alignment.get(), 16);
        assert_eq!(metadata.min_size.get(), 16);

        let metadata = <glam::UVec4 as GpuLayout<Std140>>::METADATA;
        assert_eq!(metadata.alignment.get(), 16);
        assert_eq!(metadata.min_size.get(), 16);

        let metadata = <glam::IVec4 as GpuLayout<Std140>>::METADATA;
        assert_eq!(metadata.alignment.get(), 16);
        assert_eq!(metadata.min_size.get(), 16);
    }

    #[test]
    fn check_vec2_array_layout() {
        let metadata = <[glam::Vec2; 4] as GpuLayout<Std140>>::METADATA;
        assert_eq!(metadata.alignment.get(), 16);
        assert_eq!(metadata.min_size.get(), 4 * 16);
        assert_eq!(metadata.extra.stride.get(), 16);
    }
}
