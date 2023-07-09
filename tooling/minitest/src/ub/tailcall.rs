use crate::*;


fn diverging() -> Function {
    let locals = [<i32>::get_ptype()];
    let b0 = block!(exit());

    function(Ret::No, 1, &locals, &[b0])
}

#[test]
fn become_success() {
    let locals = [];

    let b0 = block!(
        Terminator::Become {
            callee: fn_ptr(1),
            arguments: list![(const_int::<i32>(7), ArgAbi::Register)],
        }
    );

    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f, diverging()]);
    dump_program(p);
    assert_stop(p);
}

#[test]
fn become_non_exist() {
    let locals = [];

    let b0 = block!(
        Terminator::Become {
            callee: fn_ptr(1),
            arguments: list![],
        }
    );

    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f]);
    dump_program(p);
    assert_ill_formed(p);
}

#[test]
fn become_arg_count() {
    let locals = [<()>::get_ptype()];

    let b0 = block!(
        storage_live(0),
        Terminator::Become {
            callee: fn_ptr(1),
            arguments: list![(const_int::<i32>(7), ArgAbi::Register),(const_int::<i32>(7), ArgAbi::Register)],
        }
    );

    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f, diverging()]);
    dump_program(p);
    assert_ub(p, "call ABI violation: number of arguments does not agree");
}

#[test]
fn become_arg_abi() {
    let locals = [];

    let b0 = block!(
        Terminator::Become {
            callee: fn_ptr(1),
            arguments: list![(const_int::<i32>(7), ArgAbi::Stack(size(4),align(4)))],
        }
    );

    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f, diverging()]);
    dump_program(p);
    assert_ub(p, "call ABI violation: argument ABI does not agree");
}

#[test]
fn become_fib() {
    fn main(n: usize) -> Function {
        let locals = [<i32>::get_ptype()];

        let b0 = block!(
            storage_live(0),
            Terminator::Call {
                callee: fn_ptr(1),
                arguments: list![
                    (const_int::<usize>(n), ArgAbi::Register),
                    (const_int::<i32>(0), ArgAbi::Register),
                    (const_int::<i32>(1), ArgAbi::Register)
                    ],
                ret: Some((local(0), ArgAbi::Register)),
                next_block: Some(BbName(Name::from_internal(1))),
            }
        );
        let b1 = block!(print(load(local(0)),2));
        let b2 = block!(exit());
        function(Ret::No, 0, &locals, &[b0,b1,b2])
    }
    fn fib() -> Function {
        let locals = [
            <i32>::get_ptype(),
            <usize>::get_ptype(),
            <i32>::get_ptype(),
            <i32>::get_ptype(),
        ];

        let b0 = block!(
            Terminator::If {
                condition: eq(load(local(1)), const_int::<usize>(0)),
                then_block: BbName(Name::from_internal(1)),
                else_block: BbName(Name::from_internal(2)),
            }
        );
        let b1 = block!(
            assign(local(0), load(local(3))),
            Terminator::Return
        );
        let b2 = block!(
            assign(
                local(1),
                sub::<usize>(
                    load(local(1)),
                    const_int::<usize>(1),
                )
            ),
            assign(
                local(2),
                add::<i32>(
                    load(local(2)),
                    load(local(3)),
                )
            ),
            Terminator::Become {
                callee: fn_ptr(1),
                arguments: list![
                    (load(local(1)), ArgAbi::Register),
                    (load(local(3)), ArgAbi::Register),
                    (load(local(2)), ArgAbi::Register)
                    ],
            },
        );
        function(Ret::Yes, 3, &locals, &[b0,b1,b2])
    }

    fn equiv_fib(n: usize, a: i32, b: i32) -> i32 {
        if n == 0 {
            b
        } else {
            equiv_fib(n - 1, b, a + b)
        }
    }
    let p = program(&[main(10),fib()]);
    dump_program(p);
    for k in 0..10 {
        let p = program(&[main(k),fib()]);
        assert!(get_stdout(p).unwrap()[0] == equiv_fib(k, 0, 1).to_string());
    }
}

