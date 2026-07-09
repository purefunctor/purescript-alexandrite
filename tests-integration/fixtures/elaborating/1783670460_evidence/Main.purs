module Main where

import Data.Eq (class Eq, eq)

data Box a = Box a

derive instance Eq a => Eq (Box a)

class (Partial, Eq a) <= Fancy a where
  fancy :: a -> Boolean

instance fancyBox :: (Partial, Eq a) => Fancy (Box a) where
  fancy _ = true

concrete = eq (Box 1) (Box 2)

generalised left right =
  { first: eq left right
  , second: eq left right
  }

usesSuperclass :: forall a. Fancy a => a -> a -> Boolean
usesSuperclass left right = eq left right
