module Main where

import Data.Functor.Contravariant (class Contravariant)

data PredicateRecord a = PredicateRecord { label :: String, run :: a -> Boolean }
derive instance Contravariant PredicateRecord
