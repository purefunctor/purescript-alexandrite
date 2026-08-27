module Effect where

import Control.Applicative (class Applicative)
import Control.Apply (class Apply)
import Control.Bind (class Bind)
import Control.Monad (class Monad)
import Data.Functor (class Functor)

foreign import data Effect :: Type -> Type

foreign import mapE :: forall value result. (value -> result) -> Effect value -> Effect result
foreign import applyE :: forall value result. Effect (value -> result) -> Effect value -> Effect result
foreign import pureE :: forall value. value -> Effect value
foreign import bindE :: forall value result. Effect value -> (value -> Effect result) -> Effect result

instance Functor Effect where
  map = mapE

instance Apply Effect where
  apply = applyE

instance Applicative Effect where
  pure = pureE

instance Bind Effect where
  bind = bindE

instance Monad Effect
