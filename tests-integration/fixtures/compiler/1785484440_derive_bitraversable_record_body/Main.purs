module Main where

import Data.Bifunctor (class Bifunctor)
import Data.Bifoldable (class Bifoldable)
import Data.Bitraversable (class Bitraversable)

data RecordPair a b = RecordPair { zeta :: b, fixed :: Int, alpha :: a }
derive instance Bifunctor RecordPair
derive instance Bifoldable RecordPair
derive instance Bitraversable RecordPair
