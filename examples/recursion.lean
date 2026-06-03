def double : Nat -> Nat := fun n : Nat => n + n
def sumTo : Nat -> Nat := fun n : Nat => match n with | zero => 0 | succ k => n + sumTo k
def repeatAdd : Nat -> Nat -> Nat := fun n : Nat => fun acc : Nat => match n with | zero => acc | succ k => repeatAdd k (acc + n)

#eval double 21
#eval sumTo 10
#eval repeatAdd 4 0
