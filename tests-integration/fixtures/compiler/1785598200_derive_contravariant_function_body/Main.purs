module Main where

import Data.Functor.Contravariant (class Contravariant)

data Predicate a = Predicate (a -> Boolean)
derive instance Contravariant Predicate
