module Main where

import Data.Eq (class Eq)
import Data.Ord (class Ord)

data Void

derive instance Eq Void
derive instance Ord Void

data Ordering = First | Second | Third

derive instance Eq Ordering
derive instance Ord Ordering
