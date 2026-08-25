module Main where

import Control.Bind (class Bind, bind)
import Effect (Effect)

foreign import constructEffect :: forall a. String -> a -> Effect a
foreign import mark :: forall a. String -> a -> a

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
