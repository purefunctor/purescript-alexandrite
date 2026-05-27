module Main where

class Test a where
  test :: a -> a

instance testRefl :: Test a where
  test x = x

instance testInt :: Test Int where
  test _ = 0
