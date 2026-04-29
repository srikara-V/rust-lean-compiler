def not : Bool -> Bool := fun b : Bool => match b with | true => false | false => true
def getOrZero : Option Nat -> Nat := fun x : Option Nat => match x with | none => 0 | some n => n
def lengthNat : List Nat -> Nat := fun xs : List Nat => match xs with | nil => 0 | cons x rest => 1 + lengthNat rest
def sumTo : Nat -> Nat := fun n : Nat => match n with | zero => 0 | succ k => n + sumTo k

#eval not false
#eval getOrZero (some Nat 5)
#eval lengthNat (cons Nat 1 (cons Nat 2 (nil Nat)))
#eval sumTo 3
