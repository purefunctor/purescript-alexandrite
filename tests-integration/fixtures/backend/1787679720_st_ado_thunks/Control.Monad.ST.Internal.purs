module Control.Monad.ST.Internal where

import Control.Applicative (class Applicative)
import Control.Apply (class Apply)
import Control.Bind (class Bind)
import Control.Monad (class Monad)
import Data.Functor (class Functor)

foreign import data Region :: Type
foreign import data ST :: Region -> Type -> Type

foreign import map_ :: forall region a b. (a -> b) -> ST region a -> ST region b
foreign import pure_ :: forall region a. a -> ST region a
foreign import bind_ :: forall region a b. ST region a -> (a -> ST region b) -> ST region b
foreign import run :: forall a. (forall region. ST region a) -> a

instance functorST :: Functor (ST region) where
  map = map_

instance applyST :: Apply (ST region) where
  apply functionAction argumentAction =
    bind_ functionAction \function ->
      bind_ argumentAction \argument ->
        pure_ (function argument)

instance applicativeST :: Applicative (ST region) where
  pure = pure_

instance bindST :: Bind (ST region) where
  bind = bind_

instance monadST :: Monad (ST region)
