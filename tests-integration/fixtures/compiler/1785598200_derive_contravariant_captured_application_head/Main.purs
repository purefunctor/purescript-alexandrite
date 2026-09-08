module Main where

import Data.Functor.Contravariant (class Contravariant)

data Triple a b c = Triple

data Captured a = Captured (Triple a Int String)
derive instance Contravariant Captured
