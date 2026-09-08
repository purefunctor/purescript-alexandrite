module Main where

import Data.Foldable (class Foldable)
import Data.Bifoldable (class Bifoldable)

data LeftDuplicate p a = LeftDuplicate (p a Int)
derive instance Bifoldable p => Foldable (LeftDuplicate p)
