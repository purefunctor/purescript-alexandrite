module Main where

import Data.Functor.Contravariant (class Contravariant)

data Empty a
derive instance Contravariant Empty
