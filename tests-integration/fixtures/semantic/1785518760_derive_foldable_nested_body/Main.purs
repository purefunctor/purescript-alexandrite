module Main where

import Data.Foldable (class Foldable)

data Compose f g a = Compose (f (g a))
derive instance (Foldable f, Foldable g) => Foldable (Compose f g)
