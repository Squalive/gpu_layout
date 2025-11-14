use super::Std140;
use crate::{AlignmentValue, ArrayMetadata, GpuLayout, GpuLayoutSize, Metadata};

impl<T, const N: usize> GpuLayout<Std140> for [T; N]
where
    T: GpuLayout<Std140>,
{
    type ExtraMetadata = ArrayMetadata;

    const METADATA: Metadata<Self::ExtraMetadata> = {
        let base_alignment = AlignmentValue::new(16);
        let el_size = T::METADATA.min_size();

        let stride = base_alignment.round_up_size(el_size);
        let el_padding = stride.get() - el_size.get();

        let size = match N {
            0 => panic!("0 sized arrays are not supported"),
            val => stride.mul(val as u64),
        };

        Metadata {
            alignment: AlignmentValue::new(base_alignment.round_up(T::METADATA.alignment().get())),
            min_size: size,
            is_pod: T::METADATA.is_pod() && el_padding == 0,
            extra: ArrayMetadata { stride, el_padding },
        }
    };
}

impl<T, const N: usize> GpuLayoutSize<Std140> for [T; N] where T: GpuLayoutSize<Std140> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_array_layout() {
        let metadata = <[f32; 3] as GpuLayout<Std140>>::METADATA;
        assert_eq!(metadata.extra.stride.get(), 16);
        assert_eq!(metadata.min_size.get(), 16 * 3);
        assert_eq!(metadata.alignment.get(), 16);

        let metadata = <[f32; 4] as GpuLayout<Std140>>::METADATA;
        assert_eq!(metadata.alignment.get(), 16);
        assert_eq!(metadata.extra.stride.get(), 16);
        assert_eq!(metadata.min_size.get(), 16 * 4);
    }
}
