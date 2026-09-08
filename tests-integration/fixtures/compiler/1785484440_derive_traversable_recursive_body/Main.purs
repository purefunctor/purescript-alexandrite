module Main where

import Data.Functor (class Functor)
import Data.Foldable (class Foldable)
import Data.Traversable (class Traversable)

data Tree a = Leaf a | Branch (Tree a) (Tree a)
derive instance Functor Tree
derive instance Foldable Tree
derive instance Traversable Tree
