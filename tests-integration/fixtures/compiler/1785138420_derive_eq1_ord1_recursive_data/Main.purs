module Main where

import Data.Eq (class Eq, class Eq1)
import Data.Ord (class Ord, class Ord1)

data List a
  = Nil
  | Cons a (List a)

derive instance Eq a => Eq (List a)
derive instance Eq1 List
derive instance Ord a => Ord (List a)
derive instance Ord1 List

data Tree a
  = Leaf a
  | Branch (Tree a) (Tree a)

derive instance Eq a => Eq (Tree a)
derive instance Eq1 Tree
derive instance Ord a => Ord (Tree a)
derive instance Ord1 Tree
