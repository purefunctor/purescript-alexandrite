module Main where

import Data.Bifunctor (class Bifunctor)
import Data.Bifoldable (class Bifoldable)
import Data.Bitraversable (class Bitraversable)

data Pair a b = Pair a Int b
derive instance Bifunctor Pair
derive instance Bifoldable Pair
derive instance Bitraversable Pair
