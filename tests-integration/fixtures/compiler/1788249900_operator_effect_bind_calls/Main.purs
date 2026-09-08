module Main where

import BindOperator ((>>=))
import Control.Applicative (pure)
import Data.Unit (Unit, unit)
import Effect (Effect)

foreign import equalInt :: Int -> Int -> Boolean
foreign import decrementInt :: Int -> Int

namedContinuation :: Unit -> Effect Int
namedContinuation _ = pure 42

namedBind :: Unit -> Effect Int
namedBind _ = pure unit >>= namedContinuation

tailBind :: Int -> Effect Int
tailBind value =
  if equalInt value 0 then pure value
  else pure unit >>= \_ -> tailBind (decrementInt value)
