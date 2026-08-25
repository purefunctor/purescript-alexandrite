module Main where

import Control.Applicative (pure)
import Control.Apply (apply)
import Data.Functor (map)
import Effect (Effect)

foreign import constructEffect :: forall a. String -> a -> Effect a
foreign import mark :: forall a. String -> a -> a

timedAdo :: String -> Effect
  { first :: String
  , second :: { seed :: String }
  }
timedAdo seed = ado
  first <- constructEffect "ado-first" seed
  second <- constructEffect "ado-second" { seed }
  in { first, second }

identity :: forall a. a -> a
identity value = value

mapped :: String -> Effect String
mapped value =
  map
    (mark "map-function" identity)
    (constructEffect "map-action" value)

applied :: String -> Effect String
applied value =
  apply
    (constructEffect "apply-function-action" identity)
    (constructEffect "apply-value-action" value)

capturedPure :: String -> Effect String
capturedPure value = pure (mark "pure-value" value)
