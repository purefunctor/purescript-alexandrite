module Main where

import Data.Foldable (class Foldable)

data Record a = Record { zeta :: a, fixed :: Int, alpha :: a }
derive instance Foldable Record
