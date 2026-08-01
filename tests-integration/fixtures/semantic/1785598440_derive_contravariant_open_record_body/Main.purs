module Main where

import Data.Functor.Contravariant (class Contravariant)

data OpenPredicate r a = OpenPredicate { run :: a -> Boolean | r }
derive instance Contravariant (OpenPredicate r)
