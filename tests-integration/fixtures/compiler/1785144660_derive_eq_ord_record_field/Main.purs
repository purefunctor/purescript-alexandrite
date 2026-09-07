module Main where

import Data.Eq (class Eq)
import Data.Ord (class Ord)

data Box = Box { value :: Int }

derive instance Eq Box
derive instance Ord Box
