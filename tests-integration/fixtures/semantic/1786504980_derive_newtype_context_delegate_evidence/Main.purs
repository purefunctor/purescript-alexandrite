module Main where

class Parent :: Type -> Constraint
class Parent value

class Relation :: Type -> Type -> Constraint
class Parent value <= Relation label value | value -> label

newtype Wrapper value = Wrapper value

derive newtype instance Relation label value => Relation label (Wrapper value)
