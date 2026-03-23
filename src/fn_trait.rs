pub trait FnOnceOutput<In, Seed> {
    type Out;
    fn call(self, x: In, s: Seed) -> Self::Out;
}

impl<In, F, O, Seed> FnOnceOutput<In, Seed> for F
where
    F: FnOnce(In, Seed) -> O,
{
    type Out = O;
    fn call(self, x: In, s: Seed) -> Self::Out {
        self(x, s)
    }
}
