module Lookalike where

class Semiring value where
  add :: value -> value -> value

foreign import intAdd :: Int -> Int -> Int

instance semiringInt :: Semiring Int where
  add = intAdd
