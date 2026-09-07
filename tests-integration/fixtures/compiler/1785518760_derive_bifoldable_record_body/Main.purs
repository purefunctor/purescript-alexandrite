module Main where

import Data.Bifoldable (class Bifoldable)

data RecordPair a b = RecordPair { zeta :: b, fixed :: Int, alpha :: a }
derive instance Bifoldable RecordPair
