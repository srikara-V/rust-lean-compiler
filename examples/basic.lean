def id : (A : Type) -> A -> A := fun A : Type => fun x : A => x
def const : (A : Type) -> (B : Type) -> A -> B -> A := fun A : Type => fun B : Type => fun x : A => fun y : B => x
def two : Nat := 1 + 1

#eval id Type Type
#eval const Type Type Type Type
#eval let x : Nat := two; x + 40
