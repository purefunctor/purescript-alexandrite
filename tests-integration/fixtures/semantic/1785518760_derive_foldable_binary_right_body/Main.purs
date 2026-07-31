module Main where

import Data.Foldable (class Foldable)

data RightNested p a = RightNested (p Int a)
derive instance Foldable (p Int) => Foldable (RightNested p)
