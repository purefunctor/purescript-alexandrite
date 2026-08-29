module Data.Ring where

class Ring value where
  negate :: value -> value

foreign import intNegate :: Int -> Int
foreign import numberNegate :: Number -> Number

instance ringInt :: Ring Int where
  negate = intNegate

instance ringNumber :: Ring Number where
  negate = numberNegate
