macro_rules! impl_matrix {
    ($c:literal, $r:literal, $type:ty, $el_ty:ty, $scalar_ty:ty) => {
        const _: () = assert!(
            2 <= $c && $c <= 4,
            "Matrix should have at least 2 columns and at most 4",
        );
        const _: () = assert!(
            2 <= $r && $r <= 4,
            "Matrix should have at least 2 rows and at most 4",
        );

        impl $crate::GpuLayout<$crate::Std140> for $type
        where
            $el_ty: $crate::GpuLayout<$crate::Std140>,
            $scalar_ty: $crate::GpuLayout<$crate::Std140>,
        {
            type ExtraMetadata = ();
            const METADATA: $crate::Metadata<Self::ExtraMetadata> = {
                let base_alignment = $crate::AlignmentValue::new(16);
                let scalar_alignment =
                    <$scalar_ty as $crate::GpuLayout<$crate::Std140>>::METADATA.alignment();
                let col_size = scalar_alignment.get() * $r;

                let alignment = $crate::AlignmentValue::new(base_alignment.round_up(col_size));
                let size = $crate::SizeValue::new(alignment.get() * $c);

                $crate::Metadata {
                    alignment,
                    min_size: size,
                    is_pod: <[$el_ty; $r] as $crate::GpuLayout<$crate::Std140>>::METADATA.is_pod(),
                    extra: (),
                }
            };
        }

        impl $crate::GpuLayoutSize<$crate::Std140> for $type
        where
            $el_ty: $crate::GpuLayoutSize<$crate::Std140>,
            $scalar_ty: $crate::GpuLayoutSize<$crate::Std140>,
        {
        }

        impl $crate::WriteInto<$crate::Std140> for $type {
            #[inline]
            fn write_into<B: $crate::BufferMut>(&self, writer: &mut $crate::Writer<B>) {
                <[$el_ty; $r] as $crate::WriteInto<$crate::Std140>>::write_into(
                    &unsafe { *(self.as_ref().as_ptr() as *const [$el_ty; $r]) },
                    writer,
                );
            }
        }

        impl $crate::ReadFrom<$crate::Std140> for $type {
            #[inline]
            fn read_from<B: $crate::BufferRef>(&mut self, reader: &mut $crate::Reader<B>) {
                let array = &mut unsafe { *(self.as_mut().as_mut_ptr() as *mut [$el_ty; $r]) };
                <[$el_ty; $r] as $crate::ReadFrom<$crate::Std140>>::read_from(array, reader);
            }
        }

        impl $crate::CreateFrom<$crate::Std140> for $type {
            #[inline]
            fn create_from<B: $crate::BufferRef>(reader: &mut $crate::Reader<B>) -> Self {
                let array =
                    <[$el_ty; $r] as $crate::CreateFrom<$crate::Std140>>::create_from(reader);
                unsafe { *(array.as_ptr() as *const $type) }
            }
        }
    };
}

cfg_if::cfg_if! {
    if #[cfg(feature = "glam")] {
        impl_matrix!(2, 2, glam::Mat2, glam::Vec2, f32);
        impl_matrix!(3, 3, glam::Mat3, glam::Vec3, f32);
        impl_matrix!(4, 4, glam::Mat4, glam::Vec4, f32);
    }
}

#[cfg(all(test, feature = "glam"))]
mod tests {
    use crate::{GpuLayout, Std140};
    use glam::{Mat2, Mat3, Mat4};

    #[test]
    fn check_mat2_layout() {
        let metadata = <Mat2 as GpuLayout<Std140>>::METADATA;
        assert_eq!(metadata.alignment.get(), 16);
        assert_eq!(metadata.min_size.get(), 16 * 2);
    }

    #[test]
    fn check_mat3_layout() {
        let metadata = <Mat3 as GpuLayout<Std140>>::METADATA;
        assert_eq!(metadata.alignment.get(), 16);
        assert_eq!(metadata.min_size.get(), 16 * 3);
    }

    #[test]
    fn check_mat4_layout() {
        let metadata = <Mat4 as GpuLayout<Std140>>::METADATA;
        assert_eq!(metadata.alignment.get(), 16);
        assert_eq!(metadata.min_size.get(), 16 * 4);
    }

    // #[test]
    // fn check_writer_reader() {
    //     let mut buffer = [0u8; 256];
    //     let mat = Mat3::from_angle(1.0);

    //     let mut writer = Writer::new::<Std140, _>(&mat, &mut buffer, 0).unwrap();
    //     mat.write_into(&mut writer);

    //     let mut reader = Reader::new::<Std140, Mat3>(&buffer, 0).unwrap();
    //     assert_eq!(mat, Mat3::create_from(&mut reader));
    // }
}
