module Data.Ring where

import Data.Semiring (class Semiring, zero)

class Semiring value <= Ring value where
  sub :: value -> value -> value

foreign import intSub :: Int -> Int -> Int
foreign import numSub :: Number -> Number -> Number

instance ringInt :: Ring Int where
  sub = intSub

instance ringNumber :: Ring Number where
  sub = numSub

negate :: forall value. Ring value => value -> value
negate value = sub zero value
