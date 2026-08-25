module Main where

import Control.Applicative (pure)
import Control.Bind (bind, discard)
import Control.Monad.ST.Internal (ST)
import Data.Unit (Unit(..))

foreign import constructST :: forall region a. String -> a -> ST region a
foreign import mark :: forall a. String -> a -> a

chained :: forall region. String -> ST region
  { first :: String
  , second :: { first :: String }
  }
chained seed = do
  first <- constructST "first" seed
  let
    secondInput = { first }
  second <- constructST "second" secondInput
  let
    result = { first, second }
  constructST "third" result

discarded :: forall region. String -> ST region String
discarded seed = do
  constructST "discard-first" Unit
  let
    result = mark "discard-let" seed
  constructST "discard-second" result

pureAfterBind :: forall region. String -> ST region { value :: String }
pureAfterBind seed = do
  value <- constructST "pure-action" seed
  let
    result = { value }
  pure (mark "pure-body" result)
