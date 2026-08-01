module Main where

import Data.Profunctor (class Profunctor)

data Function a b = Function (a -> b)
derive instance Profunctor Function

data NestedLeft a b = NestedLeft (Function a Int)
derive instance Profunctor NestedLeft
