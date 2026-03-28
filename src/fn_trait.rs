pub trait FnOnceOutput<In> {
    type Out;
    fn call(self, x: In) -> Self::Out;
}

impl<In, F, O> FnOnceOutput<In> for F
where
    F: FnOnce(In) -> O,
{
    type Out = O;
    fn call(self, x: In) -> Self::Out {
        self(x)
    }
}
