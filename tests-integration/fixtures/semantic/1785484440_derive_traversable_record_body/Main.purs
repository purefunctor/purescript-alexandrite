module Main where

import Data.Functor (class Functor)
import Data.Foldable (class Foldable)
import Data.Traversable (class Traversable)

data Record a = Record { zeta :: a, fixed :: Int, alpha :: a }
derive instance Functor Record
derive instance Foldable Record
derive instance Traversable Record
