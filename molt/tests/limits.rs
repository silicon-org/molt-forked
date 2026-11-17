//! Deep nesting and huge sizes give normal errors (`MOLT LIMIT NESTING`, `MOLT LIMIT SIZE`)
//! instead of overflowing the stack or aborting on an allocation. The scripts use `eval`,
//! `lrepeat` and big integers, which are in the `full` profile.
#![cfg(feature = "full")]

use molt_forked::limits::{
    MAX_LIST_LENGTH, MAX_PARSE_NESTING, MAX_STRING_BYTES, NESTING, SIZE,
};
use molt_forked::Interp;

/// The stack of the thread that runs a script: a debug build takes about 10 KB per level of
/// nesting, so the default recursion limit of 1000 needs more than the 2 MB of a test thread.
const STACK: usize = 64 << 20;

/// Runs `script` and gives the message and the error code of the error it ends with, or the
/// result as the message and an empty code.
fn run(script: &'static str) -> (String, String) {
    std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(move || {
            let mut interp = Interp::default();
            match interp.eval(script) {
                Ok(v) => (v.as_str().to_owned(), String::new()),
                Err(e) => {
                    (e.value().as_str().to_owned(), e.error_code().as_str().to_owned())
                }
            }
        })
        .unwrap()
        .join()
        .unwrap()
}

/// Runs `script`, which ends with `catch {...} m o; list $m [dict get $o -errorcode]`.
fn caught(script: &'static str) -> (String, String) {
    let (out, code) = run(script);
    assert_eq!(code, "", "{out}");
    let mut interp = Interp::default();
    interp.set_scalar("out", out.as_str().into()).unwrap();
    let message = interp.eval("lindex $out 0").unwrap().as_str().to_owned();
    let error = interp.eval("lindex $out 1").unwrap().as_str().to_owned();
    (message, error)
}

#[test]
fn nested_parentheses_built_at_run_time_are_a_nesting_error() {
    let (m, code) = caught(
        "set s \"[string repeat ( 50000]1[string repeat ) 50000]\"
         catch {expr $s} m o
         list $m [dict get $o -errorcode]",
    );
    assert_eq!(code, NESTING, "{m}");
    assert!(m.contains("more than 1000 nested"), "{m}");
}

#[test]
fn nested_unary_operators_are_a_nesting_error() {
    let (m, code) = caught(
        "set s \"[string repeat ! 200000]1\"
         catch {expr $s} m o
         list $m [dict get $o -errorcode]",
    );
    assert_eq!(code, NESTING, "{m}");
}

#[test]
fn nested_command_substitutions_are_a_nesting_error() {
    let (m, code) = caught(
        "set s \"set x [string repeat {[list } 30000]1[string repeat \\] 30000]\"
         catch {eval $s} m o
         list $m [dict get $o -errorcode]",
    );
    assert_eq!(code, NESTING, "{m}");
    assert_eq!(m, format!("more than {MAX_PARSE_NESTING} nested command substitutions"));
}

#[test]
fn nesting_below_the_limits_still_works() {
    let (out, code) =
        run("set s \"[string repeat ( 400][string repeat - 400]7[string repeat ) 400]\"
         set t \"set x [string repeat {[list } 900]1[string repeat \\] 900]\"
         eval $t
         list [expr $s] [llength $x]");
    assert_eq!(code, "", "{out}");
    assert_eq!(out, "7 1");
}

#[test]
fn a_string_or_list_from_a_huge_count_is_a_size_error() {
    for (script, what) in [
        ("string repeat abcd 1000000000000", "string repeat"),
        ("lrepeat 1000000000000 a", "lrepeat"),
        ("lrepeat 200000000 a b", "lrepeat"),
    ] {
        let (m, code) = run(script);
        assert_eq!(code, SIZE, "{script}: {m}");
        assert!(m.starts_with(what), "{m}");
    }
    let (m, _) = run("string repeat abcd 1000000000000");
    assert_eq!(
        m,
        format!("string repeat makes a string longer than {MAX_STRING_BYTES} bytes")
    );
    let (m, _) = run("lrepeat 1000000000000 a");
    assert_eq!(m, format!("lrepeat makes a list longer than {MAX_LIST_LENGTH} elements"));
    // Within the limits the commands work as before.
    assert_eq!(run("string repeat ab 3").0, "ababab");
    assert_eq!(run("lrepeat 2 a b").0, "a b a b");
}

#[test]
fn a_huge_integer_is_a_size_error() {
    for script in [
        "expr {(2**64)**4000000000}",
        "expr {1 << 100000000000}",
        "expr {(2**64) << 20000000000}",
    ] {
        let (m, code) = run(script);
        assert_eq!(code, SIZE, "{script}: {m}");
        assert!(m.contains("makes an integer longer than"), "{m}");
    }
    assert_eq!(run("expr {2**100}").0, "1267650600228229401496703205376");
    assert_eq!(run("expr {1 << 70}").0, "1180591620717411303424");
}

#[test]
fn a_caught_limit_leaves_the_interpreter_usable() {
    let (out, code) = run("set s \"[string repeat ( 5000]1[string repeat ) 5000]\"
         catch {expr $s}
         catch {expr $s}
         proc f {n} { if {$n > 0} { f [expr {$n - 1}] } else { return done } }
         f 200");
    assert_eq!(code, "", "{out}");
    assert_eq!(out, "done");
}
