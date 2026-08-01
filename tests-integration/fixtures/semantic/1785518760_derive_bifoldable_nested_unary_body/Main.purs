module Main where

import Data.Foldable (class Foldable)
import Data.Bifoldable (class Bifoldable)

data Wrap f g a b = Wrap (f a) (g b)
derive instance (Foldable f, Foldable g) => Bifoldable (Wrap f g)
