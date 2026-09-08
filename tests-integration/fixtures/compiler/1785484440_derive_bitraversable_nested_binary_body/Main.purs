module Main where

import Data.Bifunctor (class Bifunctor)
import Data.Bifoldable (class Bifoldable)
import Data.Bitraversable (class Bitraversable)

data Nested p a b = Nested (p a b)
derive instance Bifunctor p => Bifunctor (Nested p)
derive instance Bifoldable p => Bifoldable (Nested p)
derive instance Bitraversable p => Bitraversable (Nested p)
