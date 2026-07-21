module Main where

class Semigroup a where
  append :: a -> a -> a

test value = append value (test value)
