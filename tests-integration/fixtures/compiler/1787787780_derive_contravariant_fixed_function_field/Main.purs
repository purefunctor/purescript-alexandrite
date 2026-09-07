module Main where

import Data.Functor.Contravariant (class Contravariant)

data PredicateWithFormatter a = PredicateWithFormatter (a -> Boolean) (Int -> String)
derive instance Contravariant PredicateWithFormatter
