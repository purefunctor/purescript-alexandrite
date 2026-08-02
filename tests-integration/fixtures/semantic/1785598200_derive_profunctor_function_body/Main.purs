module Main where

import Data.Profunctor (class Profunctor)

data Function a b = Function (a -> b)
derive instance Profunctor Function
