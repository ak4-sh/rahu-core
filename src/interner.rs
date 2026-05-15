use rapidhash::RapidHashMap;
use std::collections::hash_map::Entry;
use std::num::NonZeroU32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol(NonZeroU32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalSymbol(NonZeroU32);

impl LocalSymbol {
    #[inline]
    fn from_index(idx: u32) -> Self {
        // SAFETY: idx + 1 is non-zero for any u32 idx < u32::MAX.
        // try_from in callers guards against overflow.
        Self(NonZeroU32::new(idx + 1).expect("symbol index overflow"))
    }

    #[inline]
    pub fn index(self) -> usize {
        (self.0.get() - 1) as usize
    }
}

impl Symbol {
    #[inline]
    fn from_index(idx: u32) -> Self {
        Self(NonZeroU32::new(idx + 1).expect("symbol index overflow"))
    }

    #[inline]
    pub fn index(self) -> usize {
        (self.0.get() - 1) as usize
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolRemap {
    symbols: Vec<Symbol>,
}

impl SymbolRemap {
    pub fn get(&self, local: LocalSymbol) -> Symbol {
        self.symbols[local.index()]
    }
}

/// Global interner that stores merged outputs of all local interners
pub struct GlobalInterner<'src> {
    map: RapidHashMap<&'src str, Symbol>,
    vec: Vec<&'src str>,
}

/// Local string interner used on a per text file level
pub struct LocalInterner<'src> {
    map: RapidHashMap<&'src str, LocalSymbol>,
    vec: Vec<&'src str>,
}

pub struct BuiltinSymbols {
    pub self_: Symbol,
    pub cls: Symbol,
    pub init: Symbol,
    pub new: Symbol,
    pub call: Symbol,
    pub len: Symbol,
    pub iter: Symbol,
    pub next: Symbol,
    pub getitem: Symbol,
    pub setitem: Symbol,
    pub all: Symbol,
    pub name: Symbol,
    pub main: Symbol,
}

impl<'src> LocalInterner<'src> {
    pub fn new() -> Self {
        Self {
            map: RapidHashMap::default(),
            vec: Vec::new(),
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            map: RapidHashMap::with_capacity_and_hasher(cap, Default::default()),
            vec: Vec::with_capacity(cap),
        }
    }

    pub fn intern(&mut self, s: &'src str) -> LocalSymbol {
        match self.map.entry(s) {
            Entry::Occupied(e) => *e.get(),
            Entry::Vacant(e) => {
                let id = u32::try_from(self.vec.len()).expect("too many interned strings");
                let sym = LocalSymbol::from_index(id);
                self.vec.push(s);
                e.insert(sym);
                sym
            }
        }
    }

    pub fn resolve(&self, sym: LocalSymbol) -> &'src str {
        self.vec[sym.index()]
    }
}

impl<'src> Default for LocalInterner<'src> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'src> GlobalInterner<'src> {
    pub fn new() -> Self {
        Self {
            map: RapidHashMap::default(),
            vec: Vec::new(),
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            map: RapidHashMap::with_capacity_and_hasher(cap, Default::default()),
            vec: Vec::with_capacity(cap),
        }
    }

    pub fn intern(&mut self, s: &'src str) -> Symbol {
        match self.map.entry(s) {
            Entry::Occupied(e) => *e.get(),
            Entry::Vacant(e) => {
                let id = u32::try_from(self.vec.len()).expect("too many interned strings");
                let sym = Symbol::from_index(id);
                self.vec.push(s);
                e.insert(sym);
                sym
            }
        }
    }

    pub fn resolve(&self, sym: Symbol) -> &'src str {
        self.vec[sym.index()]
    }

    pub fn with_prelude() -> (Self, BuiltinSymbols) {
        let mut interner = Self::new();
        let symbols = BuiltinSymbols {
            self_: interner.intern("self"),
            cls: interner.intern("cls"),
            init: interner.intern("__init__"),
            new: interner.intern("__new__"),
            call: interner.intern("__call__"),
            len: interner.intern("__len__"),
            iter: interner.intern("__iter__"),
            next: interner.intern("__next__"),
            getitem: interner.intern("__getitem__"),
            setitem: interner.intern("__setitem__"),
            all: interner.intern("__all__"),
            name: interner.intern("__name__"),
            main: interner.intern("__main__"),
        };
        (interner, symbols)
    }

    pub fn with_capacity_and_prelude(capacity: usize) -> (Self, BuiltinSymbols) {
        let mut interner = Self::with_capacity(capacity);
        let symbols = BuiltinSymbols {
            self_: interner.intern("self"),
            cls: interner.intern("cls"),
            init: interner.intern("__init__"),
            new: interner.intern("__new__"),
            call: interner.intern("__call__"),
            len: interner.intern("__len__"),
            iter: interner.intern("__iter__"),
            next: interner.intern("__next__"),
            getitem: interner.intern("__getitem__"),
            setitem: interner.intern("__setitem__"),
            all: interner.intern("__all__"),
            name: interner.intern("__name__"),
            main: interner.intern("__main__"),
        };
        (interner, symbols)
    }

    pub fn merge_local(&mut self, local: &LocalInterner<'src>) -> SymbolRemap {
        let symbols = local.vec.iter().copied().map(|s| self.intern(s)).collect();
        SymbolRemap { symbols }
    }
}

impl<'src> Default for GlobalInterner<'src> {
    fn default() -> Self {
        Self::new()
    }
}
