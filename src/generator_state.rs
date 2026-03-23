#[derive(Debug)]
pub enum GeneratorState<Y, R> {
    Yield(Y),
    Complete(R),
}

impl<Y, R> GeneratorState<Y, R> {
    pub fn map_yield<U>(self, f: impl FnOnce(Y) -> U) -> GeneratorState<U, R> {
        match self {
            GeneratorState::Yield(y) => GeneratorState::Yield(f(y)),
            GeneratorState::Complete(r) => GeneratorState::Complete(r),
        }
    }

    pub fn map_complete<U>(self, f: impl FnOnce(R) -> U) -> GeneratorState<Y, U> {
        match self {
            GeneratorState::Yield(y) => GeneratorState::Yield(y),
            GeneratorState::Complete(r) => GeneratorState::Complete(f(r)),
        }
    }
}
