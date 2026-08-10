module Main where

class Value :: Type -> Constraint
class Value a where
  value :: a

test :: Value Int => Int
test = (value :: Value Int => Int)
