module Lookalike where

class Semiring value where
  add :: value -> value -> value

foreign import intAdd :: Int -> Int -> Int

instance semiringInt :: Semiring Int where
  add = intAdd

negate :: Int -> Int
negate value = value
