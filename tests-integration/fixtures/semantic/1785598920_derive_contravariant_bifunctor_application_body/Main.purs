module Main where

import Data.Bifunctor (class Bifunctor)
import Data.Functor.Contravariant (class Contravariant)

data Pair a b = Pair a b
derive instance Bifunctor Pair

data Nested a = Nested (Pair (a -> Int) String)
derive instance Contravariant Nested
