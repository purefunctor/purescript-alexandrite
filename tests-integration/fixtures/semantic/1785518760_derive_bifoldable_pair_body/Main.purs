module Main where

import Data.Bifoldable (class Bifoldable)

data Pair a b = Pair a Int b
derive instance Bifoldable Pair
