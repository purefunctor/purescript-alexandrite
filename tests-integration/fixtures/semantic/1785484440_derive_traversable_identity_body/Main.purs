module Main where

import Data.Functor (class Functor)
import Data.Foldable (class Foldable)
import Data.Traversable (class Traversable)

data Identity a = Identity a
derive instance Functor Identity
derive instance Foldable Identity
derive instance Traversable Identity
