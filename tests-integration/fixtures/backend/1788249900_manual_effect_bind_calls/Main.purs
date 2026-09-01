module Main where

import Control.Applicative (pure)
import Control.Bind (bind)
import Data.Unit (Unit(..))
import Effect (Effect)

foreign import equalInt :: Int -> Int -> Boolean
foreign import decrementInt :: Int -> Int

namedContinuation :: Unit -> Effect Int
namedContinuation _ = pure 42

namedBind :: Unit -> Effect Int
namedBind _ = bind (pure Unit) namedContinuation

tailBind :: Int -> Effect Int
tailBind value =
  if equalInt value 0 then pure value
  else bind (pure Unit) (\_ -> tailBind (decrementInt value))
