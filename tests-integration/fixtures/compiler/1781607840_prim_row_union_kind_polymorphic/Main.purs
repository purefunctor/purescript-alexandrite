module Main where

import Prim.Row as Row
import Type.Proxy (Proxy(..))

foreign import data Effect :: Type -> Type

openLeftHigherKinded
  :: forall tail output
   . Row.Union (effect :: Effect | tail) () output
  => Proxy output
openLeftHigherKinded = Proxy

forceSolve
  :: forall tail output
   . Row.Union tail () output
  => { openLeftHigherKinded :: Proxy (effect :: Effect | output) }
forceSolve =
  { openLeftHigherKinded }
