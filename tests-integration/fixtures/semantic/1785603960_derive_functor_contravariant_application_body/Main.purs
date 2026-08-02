module Main where

import Data.Functor (class Functor)
import Data.Functor.Contravariant (class Contravariant)

data Predicate a = Predicate (a -> Boolean)

derive instance Contravariant Predicate

data DoubleNegative a = DoubleNegative (Predicate (Predicate a))

derive instance Functor DoubleNegative
