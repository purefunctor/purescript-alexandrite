module Main where

import Data.Bitraversable (class Bitraversable)
import Data.Foldable (class Foldable)

data RightNested p a = RightNested (p Int a)

derive instance Bitraversable p => Foldable (RightNested p)
