use std::ops::{Div, Mul};

#[derive(Copy, Clone, Debug, Default)]
pub struct Dim1(pub f32);

impl Mul<Dim1> for f32 {
    type Output = Dim1;

    fn mul(self, rhs: Dim1) -> Self::Output {
        Dim1(self * rhs.0)
    }
}

impl Mul<f32> for Dim1 {
    type Output = Dim1;

    fn mul(self, rhs: f32) -> Self::Output {
        Dim1(self.0 * rhs)
    }
}

impl Mul<Self> for Dim1 {
    type Output = Dim2;

    fn mul(self, rhs: Self) -> Self::Output {
        Dim2(self.0 * rhs.0)
    }
}

impl Div<Self> for Dim1 {
    type Output = f32;

    fn div(self, rhs: Self) -> Self::Output {
        self.0 / rhs.0
    }
}

impl Div<f32> for Dim1 {
    type Output = Dim1;

    fn div(self, rhs: f32) -> Self::Output {
        Dim1(self.0 / rhs)
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Dim2(pub f32);

impl Mul<f32> for Dim2 {
    type Output = Dim2;

    fn mul(self, rhs: f32) -> Self::Output {
        Dim2(self.0 * rhs)
    }
}

impl Div<f32> for Dim2 {
    type Output = Dim2;

    fn div(self, rhs: f32) -> Self::Output {
        Dim2(self.0 / rhs)
    }
}

impl Mul<Dim2> for f32 {
    type Output = Dim2;

    fn mul(self, rhs: Dim2) -> Self::Output {
        Dim2(rhs.0 * self)
    }
}

impl Div<Self> for Dim2 {
    type Output = f32;

    fn div(self, rhs: Self) -> Self::Output {
        self.0 / rhs.0
    }
}
