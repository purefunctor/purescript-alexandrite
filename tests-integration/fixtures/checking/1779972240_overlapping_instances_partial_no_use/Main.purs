module Main where

data Pair a b = Pair a b

class Test a where
  test :: a -> Int

instance testLeft :: Test (Pair a Int) where
  test _ = 0

instance testRight :: Test (Pair String b) where
  test _ = 1
