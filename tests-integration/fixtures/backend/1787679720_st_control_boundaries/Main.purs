module Main where

import Control.Bind (class Bind, bind)
import Control.Monad.ST.Internal (ST)

foreign import constructST :: forall region a. String -> a -> ST region a
foreign import mark :: forall a. String -> a -> a

branched :: forall region. Boolean -> String -> ST region String
branched choose seed = do
  value <- constructST "branch-action" seed
  if choose then
    constructST "branch-then" value
  else
    constructST "branch-else" value

patternLet :: forall region. String -> ST region String
patternLet seed = do
  value <- constructST "pattern-action" seed
  let
    { selected } = { selected: value }
  constructST "pattern-result" selected

genericBind :: forall m a b. Bind m => m a -> (a -> m b) -> m b
genericBind = bind

aliased :: forall region. String -> ST region String
aliased seed =
  genericBind
    (constructST "alias-first" seed)
    (\value -> constructST "alias-second" value)

deferredST :: forall region. ST region String
deferredST = do
  value <- constructST "deferred-action" "ignored"
  constructST "deferred-result" deferredValue

deferredValue :: String
deferredValue = mark "deferred-value" "deferred"
