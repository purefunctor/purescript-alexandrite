module Main where

import Data.Eq as Eq

compareArraysOnce :: Array Int -> Array Int -> Boolean
compareArraysOnce left right = Eq.eq left right
