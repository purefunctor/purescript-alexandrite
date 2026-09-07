module Main where

import Data.Bifunctor (class Bifunctor)

data RightOnly a b = RightOnly b

derive instance Bifunctor RightOnly

data NestedRightOnly a b = NestedRightOnly (RightOnly Int b)

derive instance Bifunctor NestedRightOnly
