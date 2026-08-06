module Main where

class Choose a where
  choose :: Boolean -> a -> a -> a
  identity :: a -> a

instance Choose Int where
  choose true left _ = left
  identity value = value
  choose false _ right = right
