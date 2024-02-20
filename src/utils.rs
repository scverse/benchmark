use tap::Pipe;

pub(crate) trait PipeMap: Pipe {
    #[inline(always)]
    fn pipe_map<O>(self, option: Option<O>, func: impl FnOnce(Self, O) -> Self) -> Self
    where
        Self: Sized,
        O: Sized,
    {
        if let Some(inner) = option {
            func(self, inner)
        } else {
            self
        }
    }
}

impl<T: Pipe> PipeMap for T {}
