module Main where

import IndexedDo as Indexed

foreign import data Box :: Type -> Type

data Pair a b = Pair a b

class Rebindable f where
  bind :: forall a b. f a -> (a -> f b) -> f b
  discard :: forall a b. f a -> (a -> f b) -> f b
  pure :: forall a. a -> f a

foreign import bindBox :: forall a b. Box a -> (a -> Box b) -> Box b
foreign import discardBox :: forall a b. Box a -> (a -> Box b) -> Box b
foreign import pureBox :: forall a. a -> Box a

instance rebindableBox :: Rebindable Box where
  bind = bindBox
  discard = discardBox
  pure = pureBox

foreign import boxedInt :: Box Int
foreign import boxedString :: Box String

test = do
  value <- boxedInt
  boxedString
  pure (Pair value "done")

indexed :: Indexed.Render Indexed.Start (Indexed.Use2 (Indexed.Use1 Indexed.Start)) Indexed.Unit
indexed = Indexed.do
  first <- Indexed.use1
  Indexed.use2
  Indexed.pure first
