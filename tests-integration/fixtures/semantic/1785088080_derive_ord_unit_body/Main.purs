module Main where

import Data.Eq (class Eq)
import Data.Ord (class Ord)

data Unit = Unit

derive instance Eq Unit
derive instance Ord Unit
