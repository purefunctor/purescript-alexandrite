module Main where

import Data.Eq (class Eq, class Eq1, eq)
import Prim.TypeError (class Fail, Text)

data Box a = Box a

derive instance Eq a => Eq (Box a)

derive instance Eq1 Box

class (Partial, Eq a) <= Fancy a where
  fancy :: a -> Boolean
  fancyAgain :: a -> Boolean

instance fancyBox :: (Partial, Eq a) => Fancy (Box a) where
  fancy _ = true
  fancyAgain value = fancy value

concrete = eq (Box 1) (Box 2)

generalised left right =
  { first: eq left right
  , second: eq left right
  }

usesSuperclass :: forall a. Fancy a => a -> a -> Boolean
usesSuperclass left right = eq left right

class C a where
  member :: a

outer :: forall a. C a => { inner :: C a => a }
outer = { inner: member }

siblings :: forall a. C a => { left :: C a => a, right :: C a => a }
siblings = { left: member, right: member }

boom :: Fail (Text "expected failure") => Int
boom = 1

useFailure = boom
