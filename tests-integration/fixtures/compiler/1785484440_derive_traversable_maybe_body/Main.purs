module Main where

import Data.Functor (class Functor)
import Data.Foldable (class Foldable)
import Data.Traversable (class Traversable)

data Maybe a = Nothing | Just a
derive instance Functor Maybe
derive instance Foldable Maybe
derive instance Traversable Maybe
