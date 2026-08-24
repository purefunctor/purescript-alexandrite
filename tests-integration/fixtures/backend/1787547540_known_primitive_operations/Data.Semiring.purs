module Data.Semiring where

class Semiring value where
  add :: value -> value -> value
  mul :: value -> value -> value

foreign import intAdd :: Int -> Int -> Int
foreign import intMultiply :: Int -> Int -> Int

instance semiringInt :: Semiring Int where
  add = intAdd
  mul = intMultiply
