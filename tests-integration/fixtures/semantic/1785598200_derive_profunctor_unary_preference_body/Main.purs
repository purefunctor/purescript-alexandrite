module Main where

import Data.Functor (class Functor)
import Data.Profunctor (class Profunctor)

data Function a b = Function (a -> b)
derive instance Profunctor Function
derive instance Functor (Function a)

data NestedRight a b = NestedRight (Function Int b)
derive instance Profunctor NestedRight
