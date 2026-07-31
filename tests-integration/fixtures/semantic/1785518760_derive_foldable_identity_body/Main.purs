module Main where

import Data.Foldable (class Foldable)

data Identity a = Identity a
derive instance Foldable Identity
