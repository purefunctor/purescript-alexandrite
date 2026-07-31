module Main where

import Data.Foldable (class Foldable)

data Product a = Product a Int a a
derive instance Foldable Product
