module Main where

import Data.Functor (class Functor)
import Data.Foldable (class Foldable)
import Data.Traversable (class Traversable)

data Product a = Product a Int a
derive instance Functor Product
derive instance Foldable Product
derive instance Traversable Product
