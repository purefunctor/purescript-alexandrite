module Main where

class C a where
  value :: a

instance C Int where
  value = 0

test :: Int -> Int
test input = case input of
  (output :: C Int => Int) -> output
