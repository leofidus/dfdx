use crate::gradients::{Gradients, OwnedTape, Tape};
use crate::shapes::{Dtype, Rank0};
use crate::tensor::{DeviceStorage, OneFillStorage, SplitTape, Tensor};

/// Runs backprop algorithm with all operations contained in the tape that `t` has.
///
/// This function takes ownership of `self` and returns [Gradients].
pub trait Backward<D: DeviceStorage>: Sized {
    /// Runs backprop
    fn backward(self) -> Gradients<D> {
        self.try_backward().unwrap()
    }
    /// Fallible version of [Backward::backward]
    fn try_backward(self) -> Result<Gradients<D>, D::Err>;
}

impl<E: Dtype, D: OneFillStorage<E>> Backward<D> for Tensor<Rank0, E, D, OwnedTape<D>> {
    fn try_backward(self) -> Result<Gradients<D>, D::Err> {
        let (t, mut tape) = self.split_tape();
        tape.add_backward_op(move |grads| t.device.try_fill_with_ones(grads.get_mut(&t)));
        tape.0.execute()
    }
}
