use crate::ast::*;
use crate::diag::Diagnostic;
use crate::span::Span;
use crate::token::{Token, TokenKind};

pub fn parse(tokens: &[Token]) -> Result<Program, Diagnostic> {
    let mut p = Parser { tokens, pos: 0 };
    p.parse_program()
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> &TokenKind {
        self.tokens
            .get(self.pos)
            .map(|t| &t.kind)
            .unwrap_or(&TokenKind::Eof)
    }

    fn peek_token(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn span(&self) -> Span {
        self.peek_token().span
    }

    fn at(&self, kind: &TokenKind) -> bool {
        self.peek() == kind
    }

    fn bump(&mut self) -> Token {
        let t = self.tokens[self.pos.min(self.tokens.len() - 1)].clone();
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    fn eat(&mut self, kind: TokenKind, msg: &str) -> Result<Token, Diagnostic> {
        if self.peek() == &kind {
            Ok(self.bump())
        } else {
            Err(Diagnostic::new(self.span(), msg))
        }
    }

    fn eat_ident(&mut self, msg: &str) -> Result<(String, Span), Diagnostic> {
        match self.peek().clone() {
            TokenKind::Ident(s) => {
                let span = self.span();
                self.bump();
                Ok((s, span))
            }
            TokenKind::SelfValue => {
                let span = self.span();
                self.bump();
                Ok(("self".into(), span))
            }
            _ => Err(Diagnostic::new(self.span(), msg)),
        }
    }

    fn skip_semi(&mut self) {
        while self.at(&TokenKind::Semi) {
            self.bump();
        }
    }

    fn parse_program(&mut self) -> Result<Program, Diagnostic> {
        let mut items = Vec::new();
        self.skip_semi();
        while !self.at(&TokenKind::Eof) {
            items.push(self.parse_item()?);
            self.skip_semi();
        }
        Ok(Program { items })
    }

    fn parse_item(&mut self) -> Result<Item, Diagnostic> {
        match self.peek() {
            TokenKind::Fn => Ok(Item::Fn(self.parse_fn()?)),
            TokenKind::Struct => Ok(Item::Struct(self.parse_struct()?)),
            TokenKind::Enum => Ok(Item::Enum(self.parse_enum()?)),
            TokenKind::Impl => Ok(Item::Impl(self.parse_impl()?)),
            TokenKind::Mod => {
                let span = self.span();
                self.bump();
                let (name, _) = self.eat_ident("預期模組名稱")?;
                self.skip_semi();
                Ok(Item::Mod { name, span })
            }
            _ => Err(Diagnostic::new(
                self.span(),
                "預期 `fn`、`struct`、`enum`、`impl` 或 `mod`",
            )),
        }
    }

    fn parse_fn(&mut self) -> Result<FnDef, Diagnostic> {
        let start = self.eat(TokenKind::Fn, "預期 `fn`")?;
        let (name, _) = self.eat_ident("預期函式名稱")?;
        self.eat(TokenKind::LParen, "預期 `(`")?;
        let mut params = Vec::new();
        if !self.at(&TokenKind::RParen) {
            loop {
                params.push(self.parse_param()?);
                if self.at(&TokenKind::Comma) {
                    self.bump();
                    if self.at(&TokenKind::RParen) {
                        break;
                    }
                    continue;
                }
                break;
            }
        }
        self.eat(TokenKind::RParen, "預期 `)`")?;
        let ret = if self.at(&TokenKind::Arrow) {
            self.bump();
            self.parse_type()?
        } else {
            TypeExpr::Unit(self.span())
        };
        let body = self.parse_block_expr()?;
        Ok(FnDef {
            name,
            params,
            ret,
            body,
            span: start.span,
        })
    }

    fn parse_param(&mut self) -> Result<Param, Diagnostic> {
        let span = self.span();
        if self.at(&TokenKind::And) {
            self.bump();
            let mutable = if self.at(&TokenKind::Mut) {
                self.bump();
                true
            } else {
                false
            };
            if self.at(&TokenKind::SelfValue) {
                self.bump();
                return Ok(Param {
                    name: "self".into(),
                    ty: TypeExpr::Named("Self".into(), vec![], span),
                    is_mut: mutable,
                    kind: if mutable {
                        ParamKind::SelfRefMut
                    } else {
                        ParamKind::SelfRef
                    },
                    span,
                });
            }
            let (name, _) = self.eat_ident("預期參數名稱")?;
            self.eat(TokenKind::Colon, "預期 `:`")?;
            let inner = self.parse_type()?;
            return Ok(Param {
                name,
                ty: TypeExpr::Ref {
                    inner: Box::new(inner),
                    mutable,
                    span,
                },
                is_mut: mutable,
                kind: ParamKind::Value,
                span,
            });
        }
        if self.at(&TokenKind::Mut) {
            self.bump();
            let (name, _) = self.eat_ident("預期參數名稱")?;
            self.eat(TokenKind::Colon, "預期 `:`")?;
            let ty = self.parse_type()?;
            return Ok(Param {
                name,
                ty,
                is_mut: true,
                kind: ParamKind::Value,
                span,
            });
        }
        if self.at(&TokenKind::SelfValue) {
            self.bump();
            return Ok(Param {
                name: "self".into(),
                ty: TypeExpr::Named("Self".into(), vec![], span),
                is_mut: false,
                kind: ParamKind::SelfValue,
                span,
            });
        }
        let (name, _) = self.eat_ident("預期參數名稱")?;
        self.eat(TokenKind::Colon, "預期 `:`")?;
        let ty = self.parse_type()?;
        Ok(Param {
            name,
            ty,
            is_mut: false,
            kind: ParamKind::Value,
            span,
        })
    }

    fn parse_struct(&mut self) -> Result<StructDef, Diagnostic> {
        let start = self.eat(TokenKind::Struct, "預期 `struct`")?;
        let (name, _) = self.eat_ident("預期結構名稱")?;
        self.eat(TokenKind::LBrace, "預期 `{`")?;
        let mut fields = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let (fname, _) = self.eat_ident("預期欄位名稱")?;
            self.eat(TokenKind::Colon, "預期 `:`")?;
            let ty = self.parse_type()?;
            fields.push((fname, ty));
            if self.at(&TokenKind::Comma) || self.at(&TokenKind::Semi) {
                self.bump();
            }
        }
        self.eat(TokenKind::RBrace, "預期 `}`")?;
        Ok(StructDef {
            name,
            fields,
            span: start.span,
        })
    }

    fn parse_enum(&mut self) -> Result<EnumDef, Diagnostic> {
        let start = self.eat(TokenKind::Enum, "預期 `enum`")?;
        let (name, _) = self.eat_ident("預期列舉名稱")?;
        self.eat(TokenKind::LBrace, "預期 `{`")?;
        let mut variants = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let (vname, _) = self.eat_ident("預期變體名稱")?;
            let mut fields = Vec::new();
            if self.at(&TokenKind::LParen) {
                self.bump();
                if !self.at(&TokenKind::RParen) {
                    loop {
                        fields.push(self.parse_type()?);
                        if self.at(&TokenKind::Comma) {
                            self.bump();
                            if self.at(&TokenKind::RParen) {
                                break;
                            }
                            continue;
                        }
                        break;
                    }
                }
                self.eat(TokenKind::RParen, "預期 `)`")?;
            }
            variants.push(Variant {
                name: vname,
                fields,
            });
            if self.at(&TokenKind::Comma) || self.at(&TokenKind::Semi) {
                self.bump();
            }
        }
        self.eat(TokenKind::RBrace, "預期 `}`")?;
        Ok(EnumDef {
            name,
            variants,
            span: start.span,
        })
    }

    fn parse_impl(&mut self) -> Result<ImplDef, Diagnostic> {
        let start = self.eat(TokenKind::Impl, "預期 `impl`")?;
        let (ty, _) = self.eat_ident("預期類型名稱")?;
        self.eat(TokenKind::LBrace, "預期 `{`")?;
        let mut methods = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            methods.push(self.parse_fn()?);
            self.skip_semi();
        }
        self.eat(TokenKind::RBrace, "預期 `}`")?;
        Ok(ImplDef {
            ty,
            methods,
            span: start.span,
        })
    }

    fn parse_type(&mut self) -> Result<TypeExpr, Diagnostic> {
        let span = self.span();
        if self.at(&TokenKind::And) {
            self.bump();
            let mutable = if self.at(&TokenKind::Mut) {
                self.bump();
                true
            } else {
                false
            };
            let inner = self.parse_type()?;
            return Ok(TypeExpr::Ref {
                inner: Box::new(inner),
                mutable,
                span,
            });
        }
        if self.at(&TokenKind::LParen) {
            self.bump();
            self.eat(TokenKind::RParen, "預期 `()`")?;
            return Ok(TypeExpr::Unit(span));
        }
        let (name, nspan) = match self.peek().clone() {
            TokenKind::Ident(s) => {
                self.bump();
                (s, nspan_or(span))
            }
            _ => {
                // allow builtin names as ident only; i32 is ident
                return Err(Diagnostic::new(self.span(), "預期類型名稱"));
            }
        };
        let mut args = Vec::new();
        if self.at(&TokenKind::LBracket) {
            self.bump();
            if !self.at(&TokenKind::RBracket) {
                loop {
                    args.push(self.parse_type()?);
                    if self.at(&TokenKind::Comma) {
                        self.bump();
                        continue;
                    }
                    break;
                }
            }
            self.eat(TokenKind::RBracket, "預期 `]`")?;
        }
        Ok(TypeExpr::Named(name, args, nspan))
    }

    fn parse_block_expr(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.eat(TokenKind::LBrace, "預期 `{`")?;
        let mut stmts = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            stmts.push(self.parse_stmt()?);
            self.skip_semi();
        }
        let end = self.eat(TokenKind::RBrace, "預期 `}`")?;
        Ok(Expr::Block {
            stmts,
            span: Span {
                start: start.span.start,
                end: end.span.end,
                line: start.span.line,
                col: start.span.col,
            },
        })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        match self.peek() {
            TokenKind::Let => self.parse_let(),
            TokenKind::Return => {
                let span = self.span();
                self.bump();
                if self.at(&TokenKind::Semi) || self.at(&TokenKind::RBrace) {
                    Ok(Stmt::Return { expr: None, span })
                } else {
                    let expr = self.parse_expr()?;
                    Ok(Stmt::Return {
                        expr: Some(expr),
                        span,
                    })
                }
            }
            TokenKind::While => {
                let span = self.span();
                self.bump();
                let cond = self.parse_expr()?;
                let body = self.parse_block_expr()?;
                Ok(Stmt::While { cond, body, span })
            }
            TokenKind::For => {
                let span = self.span();
                self.bump();
                let (name, _) = self.eat_ident("預期迴圈變數")?;
                self.eat(TokenKind::In, "預期 `in`")?;
                let iter = self.parse_expr()?;
                let body = self.parse_block_expr()?;
                Ok(Stmt::For {
                    name,
                    iter,
                    body,
                    span,
                })
            }
            TokenKind::Loop => {
                let span = self.span();
                self.bump();
                let body = self.parse_block_expr()?;
                Ok(Stmt::Loop { body, span })
            }
            TokenKind::Break => {
                let span = self.span();
                self.bump();
                Ok(Stmt::Break { span })
            }
            TokenKind::Continue => {
                let span = self.span();
                self.bump();
                Ok(Stmt::Continue { span })
            }
            _ => Ok(Stmt::Expr(self.parse_expr()?)),
        }
    }

    fn parse_let(&mut self) -> Result<Stmt, Diagnostic> {
        let span = self.span();
        self.bump();
        let mutable = if self.at(&TokenKind::Mut) {
            self.bump();
            true
        } else {
            false
        };
        let (name, _) = self.eat_ident("預期變數名稱")?;
        let ty = if self.at(&TokenKind::Colon) {
            self.bump();
            Some(self.parse_type()?)
        } else {
            None
        };
        self.eat(TokenKind::Eq, "預期 `=`")?;
        let init = self.parse_expr()?;
        Ok(Stmt::Let {
            name,
            mutable,
            ty,
            init,
            span,
        })
    }

    fn parse_expr(&mut self) -> Result<Expr, Diagnostic> {
        self.parse_assign()
    }

    fn parse_assign(&mut self) -> Result<Expr, Diagnostic> {
        let left = self.parse_or()?;
        if self.at(&TokenKind::Eq) {
            let span = self.span();
            self.bump();
            let right = self.parse_assign()?;
            return Ok(Expr::Assign {
                place: Box::new(left),
                value: Box::new(right),
                span,
            });
        }
        let compound = match self.peek() {
            TokenKind::PlusEq => Some(BinOp::Add),
            TokenKind::MinusEq => Some(BinOp::Sub),
            TokenKind::StarEq => Some(BinOp::Mul),
            _ => None,
        };
        if let Some(op) = compound {
            let span = self.span();
            self.bump();
            let right = self.parse_assign()?;
            return Ok(Expr::Assign {
                place: Box::new(left.clone()),
                value: Box::new(Expr::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                    span,
                }),
                span,
            });
        }
        Ok(left)
    }

    fn parse_or(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_and()?;
        while self.at(&TokenKind::OrOr) {
            let span = self.span();
            self.bump();
            let right = self.parse_and()?;
            left = Expr::Binary {
                op: BinOp::Or,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_bitor()?;
        while self.at(&TokenKind::AndAnd) {
            let span = self.span();
            self.bump();
            let right = self.parse_bitor()?;
            left = Expr::Binary {
                op: BinOp::And,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_bitor(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_bitxor()?;
        while self.at(&TokenKind::Pipe) {
            let span = self.span();
            self.bump();
            let right = self.parse_bitxor()?;
            left = Expr::Binary {
                op: BinOp::BitOr,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_bitxor(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_bitand()?;
        while self.at(&TokenKind::Caret) {
            let span = self.span();
            self.bump();
            let right = self.parse_bitand()?;
            left = Expr::Binary {
                op: BinOp::BitXor,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_bitand(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_eq()?;
        while self.at(&TokenKind::And) {
            let span = self.span();
            self.bump();
            let right = self.parse_eq()?;
            left = Expr::Binary {
                op: BinOp::BitAnd,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_eq(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_cmp()?;
        loop {
            let op = match self.peek() {
                TokenKind::EqEq => BinOp::Eq,
                TokenKind::BangEq => BinOp::Ne,
                _ => break,
            };
            let span = self.span();
            self.bump();
            let right = self.parse_cmp()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_cmp(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_range()?;
        loop {
            let op = match self.peek() {
                TokenKind::Lt => BinOp::Lt,
                TokenKind::LtEq => BinOp::Le,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::GtEq => BinOp::Ge,
                _ => break,
            };
            let span = self.span();
            self.bump();
            let right = self.parse_range()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_range(&mut self) -> Result<Expr, Diagnostic> {
        let left = self.parse_shift()?;
        if self.at(&TokenKind::DotDot) {
            let span = self.span();
            self.bump();
            let right = self.parse_shift()?;
            return Ok(Expr::Range {
                start: Box::new(left),
                end: Box::new(right),
                span,
            });
        }
        Ok(left)
    }

    fn parse_shift(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_add()?;
        loop {
            let op = match self.peek() {
                TokenKind::Shl => BinOp::Shl,
                TokenKind::Shr => BinOp::Shr,
                _ => break,
            };
            let span = self.span();
            self.bump();
            let right = self.parse_add()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_add(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_mul()?;
        loop {
            let op = match self.peek() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            let span = self.span();
            self.bump();
            let right = self.parse_mul()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_mul(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                _ => break,
            };
            let span = self.span();
            self.bump();
            let right = self.parse_unary()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, Diagnostic> {
        let span = self.span();
        if self.at(&TokenKind::Minus) {
            self.bump();
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnOp::Neg,
                expr: Box::new(expr),
                span,
            });
        }
        if self.at(&TokenKind::Bang) {
            self.bump();
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnOp::Not,
                expr: Box::new(expr),
                span,
            });
        }
        if self.at(&TokenKind::Tilde) {
            self.bump();
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnOp::BitNot,
                expr: Box::new(expr),
                span,
            });
        }
        if self.at(&TokenKind::Star) {
            self.bump();
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnOp::Deref,
                expr: Box::new(expr),
                span,
            });
        }
        if self.at(&TokenKind::And) {
            self.bump();
            let op = if self.at(&TokenKind::Mut) {
                self.bump();
                UnOp::RefMut
            } else {
                UnOp::Ref
            };
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op,
                expr: Box::new(expr),
                span,
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_primary()?;
        loop {
            let span = self.span();
            if self.at(&TokenKind::LParen) {
                self.bump();
                let args = self.parse_arg_list()?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                    span,
                };
            } else if self.at(&TokenKind::LBracket) {
                self.bump();
                let index = self.parse_expr()?;
                self.eat(TokenKind::RBracket, "預期 `]`")?;
                expr = Expr::Index {
                    base: Box::new(expr),
                    index: Box::new(index),
                    span,
                };
            } else if self.at(&TokenKind::Dot) {
                self.bump();
                let (name, _) = self.eat_ident("預期欄位或方法名稱")?;
                if self.at(&TokenKind::LParen) {
                    self.bump();
                    let args = self.parse_arg_list()?;
                    expr = Expr::Call {
                        callee: Box::new(Expr::Field {
                            base: Box::new(expr),
                            name,
                            span,
                        }),
                        args,
                        span,
                    };
                } else {
                    expr = Expr::Field {
                        base: Box::new(expr),
                        name,
                        span,
                    };
                }
            } else if self.at(&TokenKind::Question) {
                self.bump();
                expr = Expr::Try {
                    expr: Box::new(expr),
                    span,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_arg_list(&mut self) -> Result<Vec<Expr>, Diagnostic> {
        let mut args = Vec::new();
        if !self.at(&TokenKind::RParen) {
            loop {
                args.push(self.parse_expr()?);
                if self.at(&TokenKind::Comma) {
                    self.bump();
                    if self.at(&TokenKind::RParen) {
                        break;
                    }
                    continue;
                }
                break;
            }
        }
        self.eat(TokenKind::RParen, "預期 `)`")?;
        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expr, Diagnostic> {
        let span = self.span();
        match self.peek().clone() {
            TokenKind::Int { value, suffix_i64 } => {
                self.bump();
                Ok(Expr::Int {
                    value,
                    ty_i64: suffix_i64,
                    span,
                })
            }
            TokenKind::Float(value) => {
                self.bump();
                Ok(Expr::Float { value, span })
            }
            TokenKind::True => {
                self.bump();
                Ok(Expr::Bool { value: true, span })
            }
            TokenKind::False => {
                self.bump();
                Ok(Expr::Bool { value: false, span })
            }
            TokenKind::String(s) => {
                self.bump();
                Ok(Expr::Str { value: s, span })
            }
            TokenKind::LParen => {
                self.bump();
                if self.at(&TokenKind::RParen) {
                    self.bump();
                    return Ok(Expr::Unit { span });
                }
                let inner = self.parse_expr()?;
                self.eat(TokenKind::RParen, "預期 `)`")?;
                Ok(inner)
            }
            TokenKind::LBrace => self.parse_block_expr(),
            TokenKind::If => self.parse_if(),
            TokenKind::Match => self.parse_match(),
            TokenKind::SelfValue => {
                self.bump();
                Ok(Expr::Var {
                    name: "self".into(),
                    span,
                })
            }
            TokenKind::Ident(name) => {
                self.bump();
                if self.at(&TokenKind::LBrace) && looks_like_struct_lit(self) {
                    self.bump();
                    let mut fields = Vec::new();
                    while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                        let (fname, _) = self.eat_ident("預期欄位名稱")?;
                        self.eat(TokenKind::Colon, "預期 `:`")?;
                        let val = self.parse_expr()?;
                        fields.push((fname, val));
                        if self.at(&TokenKind::Comma) {
                            self.bump();
                        }
                    }
                    self.eat(TokenKind::RBrace, "預期 `}`")?;
                    return Ok(Expr::StructLit { name, fields, span });
                }
                if self.at(&TokenKind::Colon) {
                    // Enum::Variant — we already consumed ident; need ::
                }
                // Path Enum::Variant: ident then :: which is two colons
                if self.at(&TokenKind::Colon) {
                    let saved = self.pos;
                    self.bump();
                    if self.at(&TokenKind::Colon) {
                        self.bump();
                        let (variant, _) = self.eat_ident("預期變體名稱")?;
                        let args = if self.at(&TokenKind::LParen) {
                            self.bump();
                            let a = self.parse_arg_list()?;
                            a
                        } else {
                            vec![]
                        };
                        return Ok(Expr::EnumLit {
                            enum_name: Some(name),
                            variant,
                            args,
                            span,
                        });
                    } else {
                        self.pos = saved;
                    }
                }
                // Variant(...) prelude
                if self.at(&TokenKind::LParen)
                    && matches!(name.as_str(), "Some" | "Ok" | "Err" | "Share")
                {
                    self.bump();
                    let args = self.parse_arg_list()?;
                    return Ok(Expr::EnumLit {
                        enum_name: None,
                        variant: name,
                        args,
                        span,
                    });
                }
                if matches!(name.as_str(), "None") {
                    return Ok(Expr::EnumLit {
                        enum_name: Some("Option".into()),
                        variant: name,
                        args: vec![],
                        span,
                    });
                }
                Ok(Expr::Var { name, span })
            }
            _ => Err(Diagnostic::new(self.span(), "預期表達式")),
        }
    }

    fn parse_if(&mut self) -> Result<Expr, Diagnostic> {
        let span = self.span();
        self.bump();
        let cond = self.parse_expr()?;
        let then_branch = self.parse_block_expr()?;
        let else_branch = if self.at(&TokenKind::Else) {
            self.bump();
            if self.at(&TokenKind::If) {
                Some(Box::new(self.parse_if()?))
            } else {
                Some(Box::new(self.parse_block_expr()?))
            }
        } else {
            None
        };
        Ok(Expr::If {
            cond: Box::new(cond),
            then_branch: Box::new(then_branch),
            else_branch,
            span,
        })
    }

    fn parse_match(&mut self) -> Result<Expr, Diagnostic> {
        let span = self.span();
        self.bump();
        let scrutinee = self.parse_expr()?;
        self.eat(TokenKind::LBrace, "預期 `{`")?;
        let mut arms = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let pat = self.parse_pat()?;
            self.eat(TokenKind::FatArrow, "預期 `=>`")?;
            let body = self.parse_expr()?;
            arms.push(MatchArm { pat, body });
            if self.at(&TokenKind::Comma) || self.at(&TokenKind::Semi) {
                self.bump();
            }
        }
        self.eat(TokenKind::RBrace, "預期 `}`")?;
        Ok(Expr::Match {
            scrutinee: Box::new(scrutinee),
            arms,
            span,
        })
    }

    fn parse_pat(&mut self) -> Result<Pat, Diagnostic> {
        match self.peek().clone() {
            TokenKind::Ident(s) if s == "_" => {
                self.bump();
                Ok(Pat::Wildcard)
            }
            TokenKind::Int { value, .. } => {
                self.bump();
                Ok(Pat::Int(value))
            }
            TokenKind::True => {
                self.bump();
                Ok(Pat::Bool(true))
            }
            TokenKind::False => {
                self.bump();
                Ok(Pat::Bool(false))
            }
            TokenKind::String(s) => {
                self.bump();
                Ok(Pat::Str(s))
            }
            TokenKind::Ident(name) => {
                self.bump();
                let mut enum_name = None;
                let mut vname = name.clone();
                if self.at(&TokenKind::Colon) {
                    self.bump();
                    self.eat(TokenKind::Colon, "預期 `::`")?;
                    let (vn, _) = self.eat_ident("預期變體名稱")?;
                    enum_name = Some(name);
                    vname = vn;
                }
                if self.at(&TokenKind::LParen) {
                    self.bump();
                    let mut args = Vec::new();
                    if !self.at(&TokenKind::RParen) {
                        loop {
                            args.push(self.parse_pat()?);
                            if self.at(&TokenKind::Comma) {
                                self.bump();
                                if self.at(&TokenKind::RParen) {
                                    break;
                                }
                                continue;
                            }
                            break;
                        }
                    }
                    self.eat(TokenKind::RParen, "預期 `)`")?;
                    Ok(Pat::Variant {
                        enum_name,
                        name: vname,
                        args,
                    })
                } else if vname
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
                    || matches!(vname.as_str(), "None" | "Some" | "Ok" | "Err")
                {
                    Ok(Pat::Variant {
                        enum_name,
                        name: vname,
                        args: vec![],
                    })
                } else {
                    Ok(Pat::Binding(vname))
                }
            }
            _ => Err(Diagnostic::new(self.span(), "預期模式")),
        }
    }
}

fn nspan_or(span: Span) -> Span {
    span
}

/// Distinguishes `Point { x: 1 }` from a block that happens to follow an ident
/// (which shouldn't occur in expression position after an ident without call).
fn looks_like_struct_lit(p: &Parser<'_>) -> bool {
    // peek is `{`. Look at token after `{`: ident then `:`
    let i = p.pos + 1;
    match (p.tokens.get(i), p.tokens.get(i + 1)) {
        (Some(t1), Some(t2)) => {
            matches!(t1.kind, TokenKind::Ident(_)) && matches!(t2.kind, TokenKind::Colon)
        }
        _ => false,
    }
}

pub fn parse_repl_line(tokens: &[Token]) -> Result<ReplInput, Diagnostic> {
    let mut p = Parser { tokens, pos: 0 };
    match p.peek() {
        TokenKind::Fn | TokenKind::Struct | TokenKind::Enum | TokenKind::Impl | TokenKind::Mod => {
            let item = p.parse_item()?;
            Ok(ReplInput::Item(item))
        }
        TokenKind::Let
        | TokenKind::Return
        | TokenKind::While
        | TokenKind::For
        | TokenKind::Loop
        | TokenKind::Break
        | TokenKind::Continue => {
            let stmt = p.parse_stmt()?;
            Ok(ReplInput::Stmt(stmt))
        }
        TokenKind::Eof => Ok(ReplInput::Empty),
        _ => {
            let expr = p.parse_expr()?;
            Ok(ReplInput::Expr(expr))
        }
    }
}

pub enum ReplInput {
    Empty,
    Item(Item),
    Stmt(Stmt),
    Expr(Expr),
}
