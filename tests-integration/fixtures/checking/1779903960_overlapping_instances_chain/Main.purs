module Main where

class Test a where
  test :: a -> a

instance testInt :: Test Int where
  test _ = 0

else instance testRefl :: Test a where
  test x = x

value :: Int
value = test 1
