module Effect where

import Control.Applicative (class Applicative)
import Control.Apply (class Apply)
import Control.Bind (class Bind)
import Control.Monad (class Monad)
import Data.Functor (class Functor)

foreign import data Effect :: Type -> Type

foreign import mapE :: forall a b. (a -> b) -> Effect a -> Effect b
foreign import applyE :: forall a b. Effect (a -> b) -> Effect a -> Effect b
foreign import pureE :: forall a. a -> Effect a
foreign import bindE :: forall a b. Effect a -> (a -> Effect b) -> Effect b

instance functorEffect :: Functor Effect where
  map = mapE

instance applyEffect :: Apply Effect where
  apply = applyE

instance applicativeEffect :: Applicative Effect where
  pure = pureE

instance bindEffect :: Bind Effect where
  bind = bindE

instance monadEffect :: Monad Effect
