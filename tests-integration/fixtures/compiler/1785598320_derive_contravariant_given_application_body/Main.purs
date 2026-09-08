module Main where

import Data.Functor.Contravariant (class Contravariant)

data Nested f a = Nested (f a)
derive instance Contravariant f => Contravariant (Nested f)
