module Main where

import Data.Functor.Contravariant (class Contravariant)

data Constant value a = Constant value
derive instance Contravariant (Constant value)
