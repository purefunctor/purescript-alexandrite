module Main where

import Data.Bifoldable (class Bifoldable)

data Nested p a b = Nested (p a b)
derive instance Bifoldable p => Bifoldable (Nested p)
