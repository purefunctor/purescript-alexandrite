module Main where

class Equal a where
  equal :: a -> a -> Boolean

instance equalInt :: Equal Int where
  equal left right = true

infix 4 equal as ==

ordinaryConstrainedOperator :: Boolean
ordinaryConstrainedOperator = 1 == 2
