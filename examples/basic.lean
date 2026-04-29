def id : (A : Type) -> A -> A := fun A : Type => fun x : A => x
def const : (A : Type) -> (B : Type) -> A -> B -> A := fun A : Type => fun B : Type => fun x : A => fun y : B => x

#eval id Type Type
#eval const Type Type Type Type
