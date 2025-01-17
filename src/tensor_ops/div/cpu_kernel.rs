use crate::tensor_ops::cpu_kernels::{BinaryDerivative, UnaryDerivative};

impl UnaryDerivative<f32> for super::ScalarDivKernelOp<f32> {
    fn f(&self, x: &f32) -> f32 {
        x / self.scalar
    }
    fn df(&self, _: &f32) -> f32 {
        1.0 / self.scalar
    }
}

impl BinaryDerivative<f32> for super::BinaryDivKernelOp {
    #[inline(always)]
    fn f(&self, x: &f32, y: &f32) -> f32 {
        x / y
    }
    #[inline(always)]
    fn dfdx(&self, _: &f32, y: &f32) -> f32 {
        1.0 / y
    }
    #[inline(always)]
    fn dfdy(&self, x: &f32, y: &f32) -> f32 {
        -x / y.powi(2)
    }
}
