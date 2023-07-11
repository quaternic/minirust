use super::*;

// A formatted expression.
// This type is used to insert the minimal required amount of parens to make expressions unambiguous, without implementing an operator priority.
pub(super) enum FmtExpr {
    // An expression that might be ambiguous in certain contexts.
    // For example `a + b` is ambiguous in the context `a + b * c`: It might mean `(a + b) * c` or `a + (b * c)`
    NonAtomic(String),

    // An expression that is never ambiguous.
    // For example `2`, `(a + b)` or `load(_1)`.
    Atomic(String),
}

impl FmtExpr {
    // Returns the contents of this FmtExpr as-is, without wrapping in `(`, `)`.
    // Use this function in unambiguous contexts like `foo(_)`.
    pub(super) fn to_string(self) -> String {
        match self {
            FmtExpr::NonAtomic(s) => s,
            FmtExpr::Atomic(s) => s,
        }
    }

    // Wraps the expression in `(`, `)` if necessary.
    // Use this function in ambiguous contexts like `a + _`.
    pub(super) fn to_atomic_string(self) -> String {
        match self {
            // This adds parens around non-atomic expressions to make them atomic.
            FmtExpr::NonAtomic(s) => format!("({s})"),
            FmtExpr::Atomic(s) => s,
        }
    }
}

pub(super) fn fmt_place_expr(p: PlaceExpr, comptypes: &mut Vec<CompType>) -> FmtExpr {
    match p {
        PlaceExpr::Local(l) => FmtExpr::Atomic(fmt_local_name(l)),
        PlaceExpr::Deref { operand, ptype } => {
            let ptype = fmt_ptype(ptype, comptypes).to_string();
            let expr = fmt_value_expr(operand.extract(), comptypes).to_string();
            FmtExpr::Atomic(format!("deref<{ptype}>({expr})"))
        }
        PlaceExpr::Field { root, field } => {
            let root = fmt_place_expr(root.extract(), comptypes).to_atomic_string();
            // `&raw foo.bar` in Rust unambiguously means `&raw (foo.bar)`, and there is
            // no other context we have to worry about. Hence this can be atomic.
            FmtExpr::Atomic(format!("{root}.{field}"))
        }
        PlaceExpr::Index { root, index } => {
            let root = fmt_place_expr(root.extract(), comptypes).to_atomic_string();
            let index = fmt_value_expr(index.extract(), comptypes).to_string();
            // This can be considered atomic due to the same reasoning as for PlaceExpr::Field, see above.
            FmtExpr::Atomic(format!("{root}[{index}]"))
        }
    }
}
pub(super) fn fmt_call_expr(call: CallExpr, comptypes: &mut Vec<CompType>) -> String {
    let callee = match call.callee {
        CallTarget::Intrinsic(intrinsic) => {
            match intrinsic {
                Intrinsic::Exit => "exit",
                Intrinsic::PrintStdout => "print",
                Intrinsic::PrintStderr => "eprint",
                Intrinsic::Allocate => "allocate",
                Intrinsic::Deallocate => "deallocate",
                Intrinsic::Spawn => "spawn",
                Intrinsic::Join => "join",
                Intrinsic::AtomicWrite => "atomic-write",
                Intrinsic::AtomicRead => "atomic-read",
                Intrinsic::CompareExchange => "compare-exchange",
                Intrinsic::Lock(LockIntrinsic::Acquire) => "lock-acquire",
                Intrinsic::Lock(LockIntrinsic::Create) => "lock-create",
                Intrinsic::Lock(LockIntrinsic::Release) => "lock-release",
            }.to_string()
        }
        CallTarget::Function(expr) => fmt_value_expr(expr, comptypes).to_atomic_string(),
    };

    let args: Vec<String> = call.arguments.iter()
        .map(|(expr, _arg_abi)| expr)
        .map(|x| fmt_value_expr(x, comptypes).to_string())
        .collect();
    let args = args.join(", ");
    format!("{callee}({args})")
}

pub(super) fn fmt_local_name(l: LocalName) -> String {
    let id = l.0.get_internal();
    format!("_{id}")
}

pub(super) fn fmt_global_name(g: GlobalName) -> String {
    let id = g.0.get_internal();
    format!("global({id})")
}

fn fmt_constant(c: Constant) -> FmtExpr {
    match c {
        Constant::Int(int) => FmtExpr::Atomic(int.to_string()),
        Constant::Bool(b) => FmtExpr::Atomic(b.to_string()),
        Constant::GlobalPointer(relocation) => fmt_relocation(relocation),
        Constant::FnPointer(fn_name) => FmtExpr::Atomic(fmt_fn_name(fn_name)),
        Constant::Variant { .. } => panic!("enums are unsupported!"),
    }
}

pub(super) fn fmt_value_expr(v: ValueExpr, comptypes: &mut Vec<CompType>) -> FmtExpr {
    match v {
        ValueExpr::Constant(c, _ty) => fmt_constant(c),
        ValueExpr::Tuple(l, t) => {
            let (lparen, rparen) = match t {
                Type::Array { .. } => ('[', ']'),
                Type::Tuple { .. } => ('(', ')'),
                _ => panic!(),
            };
            let l: Vec<_> = l.iter().map(|x| fmt_value_expr(x, comptypes).to_string()).collect();
            let l = l.join(", ");

            FmtExpr::Atomic(format!("{lparen}{l}{rparen}"))
        }
        ValueExpr::Union {
            field,
            expr,
            union_ty,
        } => {
            let union_ty = fmt_type(union_ty, comptypes).to_string();
            let expr = fmt_value_expr(expr.extract(), comptypes).to_string();
            FmtExpr::NonAtomic(format!("{union_ty} {{ field{field}: {expr} }}"))
        }
        ValueExpr::Load {
            destructive,
            source,
        } => {
            let source = source.extract();
            let source = fmt_place_expr(source, comptypes).to_string();
            let load_name = match destructive {
                true => "move",
                false => "load",
            };
            FmtExpr::Atomic(format!("{load_name}({source})"))
        }
        ValueExpr::AddrOf {
            target,
            ptr_ty: PtrType::Raw { .. },
        } => {
            let target = target.extract();
            let target = fmt_place_expr(target, comptypes).to_atomic_string();
            FmtExpr::NonAtomic(format!("&raw {target}"))
        }
        ValueExpr::AddrOf {
            target,
            ptr_ty: PtrType::Ref { mutbl, .. },
        } => {
            let target = target.extract();
            let target = fmt_place_expr(target, comptypes).to_atomic_string();
            let mutbl = match mutbl {
                Mutability::Mutable => "mut ",
                Mutability::Immutable => "",
            };
            FmtExpr::NonAtomic(format!("&{mutbl}{target}"))
        }
        ValueExpr::AddrOf {
            target: _,
            ptr_ty: _,
        } => {
            panic!("unsupported ptr_ty for AddrOr!")
        }
        ValueExpr::UnOp { operator, operand } => {
            let operand = fmt_value_expr(operand.extract(), comptypes).to_string();
            match operator {
                UnOp::Int(UnOpInt::Neg, int_ty) => {
                    let int_ty = fmt_int_type(int_ty);
                    FmtExpr::NonAtomic(format!("-<{int_ty}>({operand})"))
                }
                UnOp::Int(UnOpInt::Cast, int_ty) => {
                    let int_ty = fmt_int_type(int_ty);
                    FmtExpr::Atomic(format!("int2int<{int_ty}>({operand})"))
                }
                UnOp::Ptr2Ptr(ptr_ty) => {
                    let ptr_ty = fmt_ptr_type(ptr_ty).to_string();
                    FmtExpr::Atomic(format!("ptr2ptr<{ptr_ty}>({operand})"))
                }
                UnOp::Ptr2Int => {
                    FmtExpr::Atomic(format!("ptr2int({operand})"))
                }
                UnOp::Int2Ptr(ptr_ty) => {
                    let ptr_ty = fmt_ptr_type(ptr_ty).to_string();
                    FmtExpr::Atomic(format!("int2ptr<{ptr_ty}>({operand})"))
                }
            }
        }
        ValueExpr::BinOp {
            operator: BinOp::Int(int_op, int_ty),
            left,
            right,
        } => {
            let int_op = match int_op {
                BinOpInt::Add => '+',
                BinOpInt::Sub => '-',
                BinOpInt::Mul => '*',
                BinOpInt::Div => '/',
                BinOpInt::Rem => '%',
            };

            let int_ty = fmt_int_type(int_ty).to_string();
            let int_op = format!("{int_op}<{int_ty}>");

            let l = fmt_value_expr(left.extract(), comptypes).to_atomic_string();
            let r = fmt_value_expr(right.extract(), comptypes).to_atomic_string();

            FmtExpr::NonAtomic(format!("{l} {int_op} {r}"))
        }
        ValueExpr::BinOp {
            operator: BinOp::IntRel(rel),
            left,
            right,
        } => {
            let rel = match rel {
                IntRel::Lt => "<",
                IntRel::Le => "<=",
                IntRel::Gt => ">",
                IntRel::Ge => ">=",
                IntRel::Eq => "==",
                IntRel::Ne => "!=",
            };

            let l = fmt_value_expr(left.extract(), comptypes).to_atomic_string();
            let r = fmt_value_expr(right.extract(), comptypes).to_atomic_string();

            FmtExpr::NonAtomic(format!("{l} {rel} {r}"))
        }
        ValueExpr::BinOp {
            operator: BinOp::PtrOffset { inbounds },
            left,
            right,
        } => {
            let offset_name = match inbounds {
                true => "offset_inbounds",
                false => "offset_wrapping",
            };
            let l = fmt_value_expr(left.extract(), comptypes).to_string();
            let r = fmt_value_expr(right.extract(), comptypes).to_string();
            FmtExpr::Atomic(format!("{offset_name}({l}, {r})"))
        }
    }
}
