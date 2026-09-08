module Main where

import Data.Functor (class Functor)
import Data.Profunctor (class Profunctor)

data Function a b = Function (a -> b)

derive instance Profunctor Function

data DoubleInput a = DoubleInput (Function (Function a Int) Int)

derive instance Functor DoubleInput
