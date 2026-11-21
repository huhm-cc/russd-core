use crate::span::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    I32,
    I64,
    Bool,
    Str,
    Unit,
    Never,
    Vec(Box<Type>),
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Range,
    Named(String),
    Ref(Box<Type>, bool),
    /// Infer hole used only during checking
    Infer(u32),
}

impl Type {
    pub fn is_copy(&self) -> bool {
        match self {
            Type::I32 | Type::I64 | Type::Bool | Type::Unit | Type::Never | Type::Range => true,
            Type::Ref(_, false) => true, // shared refs copy the reference (local only)
            _ => false,
        }
    }

    pub fn is_ref(&self) -> bool {
        matches!(self, Type::Ref(_, _))
    }

    pub fn peel_ref(&self) -> &Type {
        match self {
            Type::Ref(t, _) => t.peel_ref(),
            _ => self,
        }
    }

    pub fn display(&self) -> String {
        match self {
            Type::I32 => "i32".into(),
            Type::I64 => "i64".into(),
            Type::Bool => "bool".into(),
            Type::Str => "str".into(),
            Type::Unit => "()".into(),
            Type::Never => "!".into(),
            Type::Vec(t) => format!("Vec[{}]", t.display()),
            Type::Option(t) => format!("Option[{}]", t.display()),
            Type::Result(t, e) => format!("Result[{}, {}]", t.display(), e.display()),
            Type::Range => "Range".into(),
            Type::Named(n) => n.clone(),
            Type::Ref(t, true) => format!("&mut {}", t.display()),
            Type::Ref(t, false) => format!("&{}", t.display()),
            Type::Infer(i) => format!("_?{i}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Binding {
    pub ty: Type,
    pub mutable: bool,
    pub moved: bool,
    pub shared: u32,
    pub unique: bool,
    #[allow(dead_code)]
    pub span: Span,
}
