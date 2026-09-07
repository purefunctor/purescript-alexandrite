module Main where

import Data.Eq (class Eq)

data Unit = Unit

derive instance Eq Unit
