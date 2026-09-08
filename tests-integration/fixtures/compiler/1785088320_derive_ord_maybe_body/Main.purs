module Main where

import Data.Eq (class Eq)
import Data.Ord (class Ord)

data Maybe a
  = Nothing
  | Just a

derive instance Eq a => Eq (Maybe a)
derive instance Ord a => Ord (Maybe a)
