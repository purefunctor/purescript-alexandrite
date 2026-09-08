module Main where

import Data.Bifunctor (class Bifunctor)
import Data.Bifoldable (class Bifoldable)
import Data.Bitraversable (class Bitraversable)

data Either a b = Left a | Right b
derive instance Bifunctor Either
derive instance Bifoldable Either
derive instance Bitraversable Either
