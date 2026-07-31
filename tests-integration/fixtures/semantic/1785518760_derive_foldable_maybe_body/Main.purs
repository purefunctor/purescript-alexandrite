module Main where

import Data.Foldable (class Foldable)

data Maybe a = Nothing | Just a
derive instance Foldable Maybe
