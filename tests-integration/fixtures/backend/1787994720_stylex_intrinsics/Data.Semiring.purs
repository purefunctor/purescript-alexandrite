module Data.Semiring where

class Semiring value where
  add :: value -> value -> value
  zero :: value
  mul :: value -> value -> value
  one :: value

foreign import intAdd :: Int -> Int -> Int
foreign import intMultiply :: Int -> Int -> Int
foreign import numAdd :: Number -> Number -> Number
foreign import numMul :: Number -> Number -> Number

instance semiringInt :: Semiring Int where
  add = intAdd
  zero = 0
  mul = intMultiply
  one = 1

instance semiringNumber :: Semiring Number where
  add = numAdd
  zero = 0.0
  mul = numMul
  one = 1.0
