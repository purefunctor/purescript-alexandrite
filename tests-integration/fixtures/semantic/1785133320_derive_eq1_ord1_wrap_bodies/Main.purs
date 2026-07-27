module Main where

import Data.Eq (class Eq, class Eq1)
import Data.Ord (class Ord, class Ord1)

data Wrap f a = Wrap (f a)

derive instance (Eq1 f, Eq a) => Eq (Wrap f a)
derive instance Eq1 f => Eq1 (Wrap f)
derive instance (Ord1 f, Ord a) => Ord (Wrap f a)
derive instance Ord1 f => Ord1 (Wrap f)
