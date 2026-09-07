module Main where

import Data.Eq (class Eq)

data Box = Box

derive instance Eq Box

instance eqBox :: Eq Box where
  eq _ _ = true
