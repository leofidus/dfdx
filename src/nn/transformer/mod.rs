mod decoder;
mod encoder;
mod mha;

pub use decoder::*;
pub use encoder::*;
pub use mha::*;

use crate::{
    optim::{GradientUpdate, ParamUpdater, UnusedTensors},
    tensor::{Cpu, PutTape, SplitTape},
    tensor_ops::Device,
};

use super::{Module, ModuleMut, ResetParams};

/// **Requires Nightly** Transformer architecture as described in
/// [Attention is all you need](https://arxiv.org/abs/1706.03762).
///
/// This is comprised of a [TransformerEncoder] and a [TransformerDecoder].
///
/// Generics:
/// - `MODEL_DIM`: Size of the input features to the encoder/decoder.
/// - `NUM_HEADS`: Number of heads for [MultiHeadAttention].
/// - `NUM_ENCODER_LAYERS`: Number of [TransformerEncoderBlock] to use
/// - `NUM_DECODER_LAYERS`: Number of [TransformerDecoderBlock] to use
/// - `FF_DIM`: Feedforward hidden dimension for both encoder/decoder
///
/// **Pytorch equivalent**:
/// ```python
/// torch.nn.Transformer(
///     d_model=MODEL_DIM,
///     nhead=NUM_HEADS,
///     num_encoder_layers=NUM_ENCODER_LAYERS,
///     num_decoder_layers=NUM_DECODER_LAYERS,
///     dim_feedforward=FF_DIM,
///     batch_first=True,
/// )
/// ```
#[derive(Debug, Clone)]
pub struct Transformer<
    const MODEL_DIM: usize,
    const NUM_HEADS: usize,
    const NUM_ENCODER_LAYERS: usize,
    const NUM_DECODER_LAYERS: usize,
    const FF_DIM: usize,
    D: Device<f32> = Cpu,
> {
    pub encoder: TransformerEncoder<MODEL_DIM, NUM_HEADS, FF_DIM, NUM_ENCODER_LAYERS, D>,
    pub decoder: TransformerDecoder<MODEL_DIM, NUM_HEADS, FF_DIM, NUM_DECODER_LAYERS, D>,
}

impl<
        const M: usize,
        const H: usize,
        const EL: usize,
        const DL: usize,
        const F: usize,
        D: Device<f32>,
    > ResetParams<D, f32> for Transformer<M, H, EL, DL, F, D>
{
    fn try_build(device: &D) -> Result<Self, <D>::Err> {
        Ok(Self {
            encoder: ResetParams::try_build(device)?,
            decoder: ResetParams::try_build(device)?,
        })
    }
    fn try_reset_params(&mut self) -> Result<(), <D>::Err> {
        self.encoder.try_reset_params()?;
        self.decoder.try_reset_params()?;
        Ok(())
    }
}

impl<
        const M: usize,
        const H: usize,
        const EL: usize,
        const DL: usize,
        const F: usize,
        D: Device<f32>,
    > GradientUpdate<D, f32> for Transformer<M, H, EL, DL, F, D>
{
    fn update<U>(&mut self, updater: &mut U, unused: &mut UnusedTensors) -> Result<(), <D>::Err>
    where
        U: ParamUpdater<D, f32>,
    {
        self.encoder.update(updater, unused)?;
        self.decoder.update(updater, unused)?;
        Ok(())
    }
}

impl<
        const M: usize,
        const H: usize,
        const EL: usize,
        const DL: usize,
        const F: usize,
        D: Device<f32>,
        Src: SplitTape,
        Tgt: PutTape<Src::Tape>,
    > Module<(Src, Tgt)> for Transformer<M, H, EL, DL, F, D>
where
    TransformerEncoder<M, H, F, EL, D>: Module<Src, Output = Src>,
    TransformerDecoder<M, H, F, DL, D>: Module<
        (<Tgt as PutTape<Src::Tape>>::Output, Src::NoTape),
        Output = <Tgt as PutTape<Src::Tape>>::Output,
    >,
{
    type Output = <Tgt as PutTape<Src::Tape>>::Output;

    fn forward(&self, (src, tgt): (Src, Tgt)) -> Self::Output {
        let (mem, tape) = self.encoder.forward(src).split_tape();
        self.decoder.forward((tgt.put_tape(tape), mem))
    }
}

impl<const M: usize, const H: usize, const I: usize, const J: usize, const F: usize, D, T>
    ModuleMut<T> for Transformer<M, H, I, J, F, D>
where
    D: Device<f32>,
    Self: Module<T>,
{
    type Output = <Self as Module<T>>::Output;
    fn forward_mut(&mut self, t: T) -> Self::Output {
        self.forward(t)
    }
}

#[cfg(feature = "nightly")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        nn::{tests::SimpleUpdater, ModuleBuilder},
        shapes::*,
        tensor::*,
        tensor_ops::*,
        tests::TestDevice,
    };

    #[test]
    fn test_forward() {
        let dev = TestDevice::seed_from_u64(0);
        let mut t: Transformer<16, 4, 3, 3, 8, _> = dev.build_module();

        // unbatched
        let src = dev.sample_normal::<Rank2<7, 16>>();
        let tgt = dev.sample_normal::<Rank2<9, 16>>();
        let _: Tensor<Rank2<9, 16>, _, _, _> = t.forward_mut((src, tgt));

        // batched
        let src = dev.sample_normal::<Rank3<4, 12, 16>>();
        let tgt = dev.sample_normal::<Rank3<4, 6, 16>>();
        let _: Tensor<Rank3<4, 6, 16>, _, _, _> = t.forward_mut((src, tgt));
    }

    #[test]
    fn test_backward() {
        let dev = TestDevice::seed_from_u64(0);
        let mut t: Transformer<16, 4, 3, 3, 8, _> = dev.build_module();

        let src = dev.sample_normal::<Rank3<4, 12, 16>>();
        let tgt = dev.sample_normal::<Rank3<4, 6, 16>>();
        let out: Tensor<Rank3<4, 6, 16>, _, _, _> = t.forward_mut((src.trace(), tgt));
        let g = out.mean().backward();

        let mut gs = SimpleUpdater(g);
        let mut unused: UnusedTensors = Default::default();
        t.update(&mut gs, &mut unused).unwrap();
        assert!(unused.is_empty());
    }
}
