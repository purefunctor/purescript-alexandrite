module Main where

import Data.Bifoldable (class Bifoldable)

data ReaderPair a b = ReaderPair a (Int -> b)
derive instance Bifoldable ReaderPair
