module Data.Ring where

class Ring value where
  sub :: value -> value -> value
  negate :: value -> value

foreign import intSubtract :: Int -> Int -> Int
foreign import intNegate :: Int -> Int

instance ringInt :: Ring Int where
  sub = intSubtract
  negate = intNegate
