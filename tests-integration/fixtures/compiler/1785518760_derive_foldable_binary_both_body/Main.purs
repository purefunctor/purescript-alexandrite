module Main where

import Data.Foldable (class Foldable)
import Data.Bifoldable (class Bifoldable)

data Duplicate p a = Duplicate (p a a)
derive instance Bifoldable p => Foldable (Duplicate p)
