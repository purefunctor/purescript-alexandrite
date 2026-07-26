module Main where

import Data.Eq (class Eq)
import Data.Ord (class Ord)

data Tuple a b = Tuple a b

derive instance (Eq a, Eq b) => Eq (Tuple a b)
derive instance (Ord a, Ord b) => Ord (Tuple a b)
