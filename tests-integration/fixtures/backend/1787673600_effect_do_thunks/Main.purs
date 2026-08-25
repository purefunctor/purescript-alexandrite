module Main where

import Control.Applicative (pure)
import Control.Bind (class Bind, bind, discard)
import Data.Unit (Unit(..))
import Effect (Effect)

foreign import constructEffect :: forall a. String -> a -> Effect a
foreign import mark :: forall a. String -> a -> a

chained :: String -> Effect
  { first :: String
  , second :: { first :: String }
  }
chained seed = do
  first <- constructEffect "first" seed
  let
    secondInput = { first }
  second <- constructEffect "second" secondInput
  let
    result = { first, second }
  constructEffect "third" result

discarded :: String -> Effect String
discarded seed = do
  constructEffect "discard-first" Unit
  let
    result = mark "discard-let" seed
  constructEffect "discard-second" result

pureAfterBind :: String -> Effect { value :: String }
pureAfterBind seed = do
  value <- constructEffect "pure-action" seed
  let
    result = { value }
  pure (mark "pure-body" result)

genericBind :: forall m a b. Bind m => m a -> (a -> m b) -> m b
genericBind = bind

aliased :: String -> Effect String
aliased seed =
  genericBind
    (constructEffect "alias-first" seed)
    (\value -> constructEffect "alias-second" value)

deferredEffect :: Effect String
deferredEffect = do
  value <- constructEffect "deferred-action" "ignored"
  constructEffect "deferred-result" deferredValue

deferredValue :: String
deferredValue = mark "deferred-value" "deferred"
