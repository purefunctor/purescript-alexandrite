module Main where

foreign import data ResultA :: Type
foreign import data ResultB :: Type

data Inner :: forall left right. left -> right -> Type
data Inner left right

data Outer :: forall left right. left -> right -> Type
data Outer left right

class Test :: forall input. input -> Type -> Constraint
class Test input output | input -> output

instance Test (Outer left right) ResultA

instance Test (Outer (Inner left right) other) ResultB
