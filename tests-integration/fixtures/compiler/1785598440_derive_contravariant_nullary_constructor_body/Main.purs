module Main where

import Data.Functor.Contravariant (class Contravariant)

data OptionalPredicate a = None | Some (a -> Boolean)
derive instance Contravariant OptionalPredicate
