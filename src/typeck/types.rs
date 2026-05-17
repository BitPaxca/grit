/// Internal type representation used by the type checker.
/// These are the "resolved" types — after name resolution and inference.
#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    // Primitives
    I8, I16, I32, I64, I128,
    U8, U16, U32, U64, U128,
    Isize, Usize,
    F32, F64,
    Bool,
    Char,
    String,
    Unit,     // ()
    Never,    // ! (diverging)

    // Compound
    Tuple(Vec<Ty>),
    Array(Box<Ty>, usize),
    Slice(Box<Ty>),
    Reference { is_var: bool, inner: Box<Ty> },
    Pointer { is_var: bool, inner: Box<Ty> },

    // Named / user-defined
    Struct { name: String, fields: Vec<(String, Ty)> },
    Enum { name: String, variants: Vec<(String, VariantTy)> },
    Function { params: Vec<Ty>, ret: Box<Ty> },

    // Special
    Option(Box<Ty>),
    Result(Box<Ty>, Box<Ty>),

    // Inference placeholder — filled in during type checking
    Infer(u32),

    // Error type — produced on type errors, prevents cascading
    Error,

    // Named reference (before resolution)
    Named(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum VariantTy {
    Unit,
    Tuple(Vec<Ty>),
    Struct(Vec<(String, Ty)>),
}

impl Ty {
    pub fn is_integer(&self) -> bool {
        matches!(self, Ty::I8 | Ty::I16 | Ty::I32 | Ty::I64 | Ty::I128
            | Ty::U8 | Ty::U16 | Ty::U32 | Ty::U64 | Ty::U128
            | Ty::Isize | Ty::Usize)
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Ty::F32 | Ty::F64)
    }

    pub fn is_numeric(&self) -> bool {
        self.is_integer() || self.is_float()
    }

    pub fn from_name(name: &str) -> Option<Ty> {
        match name {
            "i8" => Some(Ty::I8), "i16" => Some(Ty::I16),
            "i32" => Some(Ty::I32), "i64" => Some(Ty::I64), "i128" => Some(Ty::I128),
            "u8" => Some(Ty::U8), "u16" => Some(Ty::U16),
            "u32" => Some(Ty::U32), "u64" => Some(Ty::U64), "u128" => Some(Ty::U128),
            "isize" => Some(Ty::Isize), "usize" => Some(Ty::Usize),
            "f32" => Some(Ty::F32), "f64" => Some(Ty::F64),
            "bool" => Some(Ty::Bool), "char" => Some(Ty::Char),
            "String" => Some(Ty::String),
            _ => None,
        }
    }

    /// Human-readable type name for error messages
    pub fn display(&self) -> String {
        match self {
            Ty::I8 => "i8".into(), Ty::I16 => "i16".into(),
            Ty::I32 => "i32".into(), Ty::I64 => "i64".into(), Ty::I128 => "i128".into(),
            Ty::U8 => "u8".into(), Ty::U16 => "u16".into(),
            Ty::U32 => "u32".into(), Ty::U64 => "u64".into(), Ty::U128 => "u128".into(),
            Ty::Isize => "isize".into(), Ty::Usize => "usize".into(),
            Ty::F32 => "f32".into(), Ty::F64 => "f64".into(),
            Ty::Bool => "bool".into(), Ty::Char => "char".into(),
            Ty::String => "String".into(), Ty::Unit => "()".into(),
            Ty::Never => "!".into(),
            Ty::Tuple(ts) => format!("({})", ts.iter().map(|t| t.display()).collect::<Vec<_>>().join(", ")),
            Ty::Array(t, n) => format!("[{}; {}]", t.display(), n),
            Ty::Slice(t) => format!("[{}]", t.display()),
            Ty::Reference { is_var, inner } => {
                if *is_var { format!("&var {}", inner.display()) } else { format!("&{}", inner.display()) }
            }
            Ty::Pointer { is_var, inner } => {
                if *is_var { format!("*var {}", inner.display()) } else { format!("*{}", inner.display()) }
            }
            Ty::Struct { name, .. } => name.clone(),
            Ty::Enum { name, .. } => name.clone(),
            Ty::Function { params, ret } => {
                format!("fn({}) -> {}", params.iter().map(|t| t.display()).collect::<Vec<_>>().join(", "), ret.display())
            }
            Ty::Option(t) => format!("{}?", t.display()),
            Ty::Result(t, e) => format!("Result<{}, {}>", t.display(), e.display()),
            Ty::Infer(id) => format!("?{}", id),
            Ty::Error => "<error>".into(),
            Ty::Named(n) => n.clone(),
        }
    }
}
