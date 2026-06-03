def isSome : Option Nat -> Bool := fun x : Option Nat => match x with | none => false | some n => true
def headOrZero : List Nat -> Nat := fun xs : List Nat => match xs with | nil => 0 | cons x rest => x

#eval isSome (some Nat 3)
#eval isSome (none Nat)
#eval headOrZero (cons Nat 7 (cons Nat 8 (nil Nat)))
#eval headOrZero (nil Nat)
