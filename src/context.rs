use crate::interner::{BuiltinSymbols, GlobalInterner};

pub struct Context<'src> {
    pub global_interner: GlobalInterner<'src>,
    pub builtins: BuiltinSymbols,
}

impl<'src> Context<'src> {
    pub fn new() -> Self {
        let (global_interner, builtins) = GlobalInterner::with_prelude();

        Self {
            global_interner,
            builtins,
        }
    }
}

impl<'src> Default for Context<'src> {
    fn default() -> Self {
        Self::new()
    }
}
