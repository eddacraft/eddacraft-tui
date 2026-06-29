(module $calc
  (import "env" "log" (func $log (param i32)))
  (type $binop (func (param i32 i32) (result i32)))
  (global $counter (mut i32) (i32.const 0))
  (func $add (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.add)
  (func $sub (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.sub)
  (export "add" (func $add))
  (export "sub" (func $sub))
  (memory $mem 1))
