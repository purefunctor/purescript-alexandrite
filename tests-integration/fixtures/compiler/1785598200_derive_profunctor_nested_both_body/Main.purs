module Main where

import Data.Profunctor (class Profunctor)

data Function a b = Function (a -> b)
derive instance Profunctor Function

data Nested a b = Nested (Function a b)
derive instance Profunctor Nested
