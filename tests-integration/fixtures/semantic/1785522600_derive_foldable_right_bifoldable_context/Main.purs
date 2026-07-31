module Main where

import Data.Bifoldable (class Bifoldable)
import Data.Foldable (class Foldable)

data RightNested p a = RightNested (p Int a)

derive instance Bifoldable p => Foldable (RightNested p)
