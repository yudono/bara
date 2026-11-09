use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum TypeAnnotation {
    Int,
    Float,
    Str,
    Bool,
    List(Box<TypeAnnotation>),
    Map(Box<TypeAnnotation>, Box<TypeAnnotation>),
    Tuple(Vec<TypeAnnotation>),
    Custom(String),
    Unit,
    Dyn,
    Inferred,
}

impl fmt::Display for TypeAnnotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeAnnotation::Int => write!(f, "i64"),
            TypeAnnotation::Float => write!(f, "f64"),
            TypeAnnotation::Str => write!(f, "String"),
            TypeAnnotation::Bool => write!(f, "bool"),
            TypeAnnotation::List(inner) => write!(f, "Vec<{inner}>"),
            TypeAnnotation::Map(k, v) => write!(f, "std::collections::HashMap<{k}, {v}>"),
            TypeAnnotation::Tuple(items) => {
                let s = items.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(", ");
                write!(f, "({s})")
            }
            TypeAnnotation::Custom(name) => write!(f, "{name}"),
            TypeAnnotation::Unit => write!(f, "()"),
            TypeAnnotation::Dyn => write!(f, "BaraDyn"),
            TypeAnnotation::Inferred => write!(f, "_"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamOwnership {
    Value,
    Ref,
    MutRef,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: Option<TypeAnnotation>,
    pub ownership: Option<ParamOwnership>,
    pub default_val: Option<Expr>,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDef {
    pub name: String,
    pub ty: Option<TypeAnnotation>,
    pub default_val: Option<Expr>,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiverKind {
    None,
    RefSelf,
    MutSelf,
    ValueSelf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnDef {
    pub name: String,
    pub params: Vec<Param>,
    pub return_ty: Option<TypeAnnotation>,
    pub body: Block,
    pub is_async: bool,
    pub receiver: ReceiverKind,
    pub is_pub: bool,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeDef {
    pub name: String,
    pub fields: Vec<FieldDef>,
    pub methods: Vec<FnDef>,
    pub is_pub: bool,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<TypeAnnotation>,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub is_pub: bool,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModDef {
    pub name: String,
    pub is_pub: bool,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Expr,
    pub guard: Option<Expr>,
    pub body: Expr,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq)]
pub struct UseStmt {
    pub path: Vec<String>,
    pub alias: Option<String>,
    pub is_pub: bool,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op_str = match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Mod => "%",
            BinaryOp::Eq => "==",
            BinaryOp::Neq => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Lte => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Gte => ">=",
            BinaryOp::And => "&&",
            BinaryOp::Or => "||",
            BinaryOp::BitAnd => "&",
            BinaryOp::BitOr => "|",
            BinaryOp::BitXor => "^",
        };
        write!(f, "{op_str}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnaryOp::Not => write!(f, "!"),
            UnaryOp::Neg => write!(f, "-"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    Lit(String),
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    Str(String),
    InterpolatedStr(Vec<StringPart>),
    Bool(bool),
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Literal, (usize, usize)),
    Ident(String, (usize, usize)),
    SelfAccess(String, (usize, usize)),
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
        span: (usize, usize),
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
        span: (usize, usize),
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: (usize, usize),
    },
    MethodCall {
        receiver: Box<Expr>,
        method: String,
        args: Vec<Expr>,
        span: (usize, usize),
    },
    MemberAccess {
        object: Box<Expr>,
        member: String,
        span: (usize, usize),
    },
    PathAccess {
        segments: Vec<String>,
        span: (usize, usize),
    },
    Index {
        collection: Box<Expr>,
        index: Box<Expr>,
        span: (usize, usize),
    },
    List(Vec<Expr>, (usize, usize)),
    Map(Vec<(Expr, Expr)>, (usize, usize)),
    Pipe {
        left: Box<Expr>,
        right: Box<Expr>,
        span: (usize, usize),
    },
    Try(Box<Expr>, (usize, usize)),
    SafeNav {
        object: Box<Expr>,
        member: String,
        span: (usize, usize),
    },
    Range {
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        inclusive: bool,
        span: (usize, usize),
    },
    Go(Box<Expr>, (usize, usize)),
    Clone(Box<Expr>, (usize, usize)),
    Borrow(Box<Expr>, bool, (usize, usize)), // (expr, is_mut, span)
    Cast {
        expr: Box<Expr>,
        target_ty: String,
        span: (usize, usize),
    },
    DynWrap {
        variant: String,
        expr: Box<Expr>,
        span: (usize, usize),
    },
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
        span: (usize, usize),
    },
    Closure {
        params: Vec<String>,
        body: Box<Expr>,
        span: (usize, usize),
    },
    Ternary {
        cond: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
        span: (usize, usize),
    },
    Block(Block, (usize, usize)),
    Tuple(Vec<Expr>, (usize, usize)),
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
        span: (usize, usize),
    },
    RawRust(String, (usize, usize)),
}

impl Expr {
    pub fn span(&self) -> (usize, usize) {
        match self {
            Expr::Literal(_, s)
            | Expr::Ident(_, s)
            | Expr::SelfAccess(_, s)
            | Expr::Binary { span: s, .. }
            | Expr::Unary { span: s, .. }
            | Expr::Call { span: s, .. }
            | Expr::MethodCall { span: s, .. }
            | Expr::MemberAccess { span: s, .. }
            | Expr::PathAccess { span: s, .. }
            | Expr::Index { span: s, .. }
            | Expr::List(_, s)
            | Expr::Map(_, s)
            | Expr::Pipe { span: s, .. }
            | Expr::Try(_, s)
            | Expr::SafeNav { span: s, .. }
            | Expr::Range { span: s, .. }
            | Expr::Go(_, s)
            | Expr::Clone(_, s)
            | Expr::Borrow(_, _, s)
            | Expr::Cast { span: s, .. }
            | Expr::DynWrap { span: s, .. }
            | Expr::Assign { span: s, .. }
            | Expr::Closure { span: s, .. }
            | Expr::Ternary { span: s, .. }
            | Expr::Block(_, s)
            | Expr::Tuple(_, s)
            | Expr::Match { span: s, .. }
            | Expr::RawRust(_, s) => *s,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Assign {
        target: Expr,
        value: Expr,
        op: Option<BinaryOp>,
        is_decl: bool,
        is_mut: bool,
        ty: Option<TypeAnnotation>,
        span: (usize, usize),
    },
    Expr(Expr),
    Return(Option<Expr>, (usize, usize)),
    If {
        cond: Expr,
        then_branch: Block,
        elif_branches: Vec<(Expr, Block)>,
        else_branch: Option<Block>,
        span: (usize, usize),
    },
    For {
        item: String,
        iter: Expr,
        body: Block,
        span: (usize, usize),
    },
    While {
        cond: Expr,
        body: Block,
        span: (usize, usize),
    },
    Match {
        expr: Expr,
        arms: Vec<MatchArm>,
        span: (usize, usize),
    },
    Break((usize, usize)),
    Continue((usize, usize)),
    RawRust(String, (usize, usize)),
}

impl Stmt {
    pub fn span(&self) -> (usize, usize) {
        match self {
            Stmt::Assign { span: s, .. }
            | Stmt::Return(_, s)
            | Stmt::If { span: s, .. }
            | Stmt::For { span: s, .. }
            | Stmt::While { span: s, .. }
            | Stmt::Match { span: s, .. }
            | Stmt::Break(s)
            | Stmt::Continue(s)
            | Stmt::RawRust(_, s) => *s,
            Stmt::Expr(e) => e.span(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq)]
pub enum JsxAttrValue {
    Lit(String),
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct JsxAttribute {
    pub name: String,
    pub value: JsxAttrValue,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq)]
pub enum JsxChild {
    Text(String),
    Element(JsxElement),
    Expr(Expr),
    If {
        cond: Expr,
        then_child: Box<JsxChild>,
        else_child: Option<Box<JsxChild>>,
    },
    For {
        item: String,
        iter: Expr,
        body: Box<JsxChild>,
    },
    /// React-style effect hook: `{useEffect(dep1, dep2) => body}`.
    /// Runs `body` on mount and whenever the debug signature of `deps`
    /// changes. Collected by codegen and emitted at the end of `update()`.
    Effect {
        deps: Vec<Expr>,
        body: Expr,
        span: (usize, usize),
    },
}

/// Name of the hidden per-view mount flag injected by the analyzer.
pub const BR_MOUNTED_FIELD: &str = "__br_mounted";
/// Prefix of hidden per-effect signature fields (`__fx_sig_0`, ...).
pub const FX_SIG_PREFIX: &str = "__fx_sig_";
/// Prefix of hidden persisted child-component fields (`__child_<Tag>_<n>`).
pub const CHILD_FIELD_PREFIX: &str = "__child_";

/// True for compiler-injected hidden fields (never emitted as constructor params).
pub fn is_hidden_field(name: &str) -> bool {
    name.starts_with("__")
}

/// True for native Brui container/control tags (as opposed to component tags
/// that name a user `type` + `view` pair).
pub fn is_builtin_brui_tag(tag: &str) -> bool {
    matches!(
        tag,
        "Panel"
            | "Col"
            | "Row"
            | "Scroll"
            | "Text"
            | "Button"
            | "Input"
            | "TextField"
            | "Checkbox"
            | "Slider"
            | "Separator"
            | "Wrap"
            | "Container"
            | "Image"
            | "Progress"
            | "Spacer"
    )
}

/// Hidden field holding a persisted child component instance.
pub fn child_field_name(tag: &str, idx: usize) -> String {
    format!("{CHILD_FIELD_PREFIX}{tag}_{idx}")
}

/// Hidden field holding the debug signature of an effect's dependency list.
pub fn fx_sig_field_name(idx: usize) -> String {
    format!("{FX_SIG_PREFIX}{idx}")
}

#[derive(Debug, Clone, PartialEq)]
pub struct JsxElement {
    pub tag: String,
    pub attributes: Vec<JsxAttribute>,
    pub children: Vec<JsxChild>,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ViewDef {
    pub type_name: String,
    pub root: JsxElement,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Use(UseStmt),
    Mod(ModDef),
    Fn(FnDef),
    Type(TypeDef),
    Enum(EnumDef),
    View(ViewDef),
    StyleBlock(String, (usize, usize)),
    TopLevelStmt(Stmt),
    RawRust(String, (usize, usize)),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Program {
    pub items: Vec<Item>,
}
