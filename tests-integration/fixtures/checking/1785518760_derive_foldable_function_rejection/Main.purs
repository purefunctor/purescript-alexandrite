module Main where

import Data.Foldable (class Foldable)

data Reader a = Reader (Int -> a)
derive instance Foldable Reader
