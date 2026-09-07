module Main where

import Control.Apply (apply)
import Data.Functor (map)
import Effect (Effect)

foreign import constructEffect :: forall a. String -> a -> Effect a

timedAdo :: String -> Effect
  { first :: String
  , second :: { seed :: String }
  }
timedAdo seed = ado
  first <- constructEffect "ado-first" seed
  second <- constructEffect "ado-second" { seed }
  in { first, second }
