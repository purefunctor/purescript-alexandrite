module Main where

import Data.Bifunctor (class Bifunctor)

data Triple x a b = Triple x a b

data Captured a b = Captured (Triple a Int b)
derive instance Bifunctor Captured

data Outer a b = Outer a b

data NestedCaptured a b = NestedCaptured (Outer (Triple a Int Boolean) b)
derive instance Bifunctor NestedCaptured
