module Main where

import Data.Bifunctor (class Bifunctor)
import Data.Profunctor (class Profunctor)

data Pair a b = Pair a b
derive instance Bifunctor Pair

data Nested a b = Nested (Pair a b)
derive instance Profunctor Nested
