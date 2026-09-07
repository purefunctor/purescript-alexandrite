module Classes where

class Parent :: Type -> Constraint
class Parent a where
  parent :: a

class Parent a <= Child a where
  child :: a
