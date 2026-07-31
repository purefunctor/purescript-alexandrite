module Main where

import Data.Functor (class Functor)
import Data.Foldable (class Foldable)
import Data.Traversable (class Traversable)

data Reader a = Reader (Int -> a)
derive instance Functor Reader

instance Foldable Reader where
  foldr _ initial _ = initial
  foldl _ initial _ = initial

derive instance Traversable Reader
