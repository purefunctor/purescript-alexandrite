module Main where

import First as First
import Second as Second

data Ordering = LT | EQ | GT

class Eq a where
  eq :: a -> a -> Boolean

class Eq a <= Ord a where
  compare :: a -> a -> Ordering

class (First.Parent a, Second.Parent a) <= Child a where
  parentDict :: a -> a
