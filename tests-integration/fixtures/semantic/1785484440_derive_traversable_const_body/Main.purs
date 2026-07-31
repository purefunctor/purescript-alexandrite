module Main where

import Data.Functor (class Functor)
import Data.Foldable (class Foldable)
import Data.Traversable (class Traversable)

data Const e a = Const e
derive instance Functor (Const e)
derive instance Foldable (Const e)
derive instance Traversable (Const e)
