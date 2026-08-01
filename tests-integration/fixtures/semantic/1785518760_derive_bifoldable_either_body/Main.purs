module Main where

import Data.Bifoldable (class Bifoldable)

data Either a b = Left a | Right b
derive instance Bifoldable Either
