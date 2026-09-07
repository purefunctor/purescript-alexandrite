module Main where

import Data.Foldable (class Foldable)

data Tree a = Leaf a | Branch (Tree a) (Tree a)
derive instance Foldable Tree
