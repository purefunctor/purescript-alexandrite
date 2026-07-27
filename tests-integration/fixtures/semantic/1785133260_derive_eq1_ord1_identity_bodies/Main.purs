module Main where

import Data.Eq (class Eq, class Eq1)
import Data.Ord (class Ord, class Ord1)

data Identity a = Identity a

derive instance Eq a => Eq (Identity a)
derive instance Eq1 Identity
derive instance Ord a => Ord (Identity a)
derive instance Ord1 Identity
