module Main where

import Data.Profunctor (class Profunctor)

data Nested p a b = Nested (p a b)
derive instance Profunctor p => Profunctor (Nested p)
