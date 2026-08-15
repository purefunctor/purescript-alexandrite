module Main where

data Pair a b = Pair a b

infixr 6 type Pair as :+:

class Convert :: Type -> Type -> Constraint
class Convert source target where
  convert :: source -> target

instance Convert Int String where
  convert _ = ""

instance Convert (Int :+: String) Boolean where
  convert _ = true
