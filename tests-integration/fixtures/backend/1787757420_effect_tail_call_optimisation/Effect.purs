module Effect where

import Control.Applicative (class Applicative)
import Control.Apply (class Apply)
import Control.Bind (class Bind)
import Control.Monad (class Monad)
import Data.Functor (class Functor)

foreign import data Effect :: Type -> Type

foreign import mapEffect :: forall a b. (a -> b) -> Effect a -> Effect b
foreign import applyEffect :: forall a b. Effect (a -> b) -> Effect a -> Effect b
foreign import pureEffect :: forall a. a -> Effect a
foreign import bindEffect :: forall a b. Effect a -> (a -> Effect b) -> Effect b

instance functorEffect :: Functor Effect where
  map = mapEffect

instance applyEffectInstance :: Apply Effect where
  apply = applyEffect

instance applicativeEffect :: Applicative Effect where
  pure = pureEffect

instance bindEffectInstance :: Bind Effect where
  bind = bindEffect

instance monadEffect :: Monad Effect
