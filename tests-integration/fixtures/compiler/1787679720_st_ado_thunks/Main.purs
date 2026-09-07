module Main where

import Control.Applicative (pure)
import Control.Apply (apply)
import Control.Monad.ST.Internal (ST)
import Data.Functor (map)

foreign import constructST :: forall region a. String -> a -> ST region a
foreign import mark :: forall a. String -> a -> a

timedAdo :: forall region. String -> ST region
  { first :: String
  , second :: { seed :: String }
  }
timedAdo seed = ado
  first <- constructST "ado-first" seed
  second <- constructST "ado-second" { seed }
  in { first, second }

identity :: forall a. a -> a
identity value = value

mapped :: forall region. String -> ST region String
mapped value =
  map
    (mark "map-function" identity)
    (constructST "map-action" value)

applied :: forall region. String -> ST region String
applied value =
  apply
    (constructST "apply-function-action" identity)
    (constructST "apply-value-action" value)

capturedPure :: forall region. String -> ST region String
capturedPure value = pure (mark "pure-value" value)
