module Main where

class Empty :: Type -> Constraint
class Empty a

class Example :: forall k. k -> Type -> Constraint
class Example value result where
  identity :: result -> result
  alternate :: forall intermediate. intermediate -> result
  constrained :: forall intermediate. Empty intermediate => intermediate -> result
