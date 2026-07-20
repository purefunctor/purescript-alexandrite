module Main where

import IndexedAdo as Indexed

foreign import data Box :: Type -> Type

data Pair a b = Pair a b

class Rebindable f where
  map :: forall a b. (a -> b) -> f a -> f b
  apply :: forall a b. f (a -> b) -> f a -> f b
  pure :: forall a. a -> f a

foreign import mapBox :: forall a b. (a -> b) -> Box a -> Box b
foreign import applyBox :: forall a b. Box (a -> b) -> Box a -> Box b
foreign import pureBox :: forall a. a -> Box a

instance rebindableBox :: Rebindable Box where
  map = mapBox
  apply = applyBox
  pure = pureBox

foreign import boxedInt :: Box Int
foreign import boxedString :: Box String

test = ado
  left <- boxedInt
  right <- boxedString
  in Pair left right

discard = ado
  boxedInt
  value <- boxedString
  in value

pureAdo = ado in 42

indexed :: Indexed.Render Indexed.Start (Indexed.Use2 (Indexed.Use1 Indexed.Start)) (Pair Indexed.Unit Indexed.Unit)
indexed = Indexed.ado
  first <- Indexed.use1
  second <- Indexed.use2
  in Pair first second

indexedPure :: forall hooks. Indexed.Render hooks hooks Int
indexedPure = Indexed.ado in 42
