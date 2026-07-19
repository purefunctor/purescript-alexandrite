module Main where

class Semigroup a where
  append :: a -> a -> a

first value = append value (second value)

second value = append value (first value)
