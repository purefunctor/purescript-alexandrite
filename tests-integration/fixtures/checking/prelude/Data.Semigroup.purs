module Data.Semigroup where

class Semigroup a where
  append :: a -> a -> a

infixr 5 append as <>
