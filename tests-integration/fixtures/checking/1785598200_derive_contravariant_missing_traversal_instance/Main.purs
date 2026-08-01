module Main where

import Data.Functor.Contravariant (class Contravariant)

data Opaque a = Opaque a

data Wrapped a = Wrapped (Opaque (a -> Int))
derive instance Contravariant Wrapped
