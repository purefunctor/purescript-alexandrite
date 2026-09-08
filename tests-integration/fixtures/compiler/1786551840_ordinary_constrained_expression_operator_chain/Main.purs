module Main where

class Semigroup a where
  append :: a -> a -> a

instance semigroupString :: Semigroup String where
  append left right = left

infixr 5 append as <>

ordinaryConstrainedOperatorChain :: String
ordinaryConstrainedOperatorChain = "first" <> "second" <> "third"
